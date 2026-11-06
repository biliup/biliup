// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// D221: an MP4/MOV video track's sample table, for the preview's scrub frame cache (issue #12 item 3).
// The renderer decodes frames itself with WebCodecs, so it needs what a demuxer knows: where each
// sample's bytes are, when it is shown, which samples are keyframes, and the codec's configuration
// record (avcC/hvcC). Only plain (non-fragmented) ISO-BMFF with H.264 or HEVC video is read;
// anything else is `undefined` and the preview keeps using <video> for it.
import { avccSps, h264Colour, hevcColour, hvccSps, type StreamColour } from "@/app/lib/editor/core/media/sps";

export type { StreamColour } from "@/app/lib/editor/core/media/sps";
//
// Presentation times follow the edit list the way ffmpeg's mov demuxer (and so Chromium's <video>
// and the export) apply it: an optional leading empty edit delays the track, then one edit's
// media_time is where the shown track starts. Several real edits aren't supported (undefined).
//
// The colour description (H.273 codes) is the sample entry's colr box where it says something, else
// the SPS's VUI, as ffmpeg's mov demuxer fills it in; {} for footage that says nothing (BT.601 to
// Chromium's <video> and to ffmpeg, BT.709 to WebCodecs: the scrub cache fixes that up).
//
// Every size in the file is untrusted: reads are bounds-checked and counts capped, so a malformed
// file is "no index", never a long walk or a huge allocation.

/** A video track's samples, in decode order. */
export interface Mp4Index {
  /** WebCodecs codec string (avc1.PPCCLL, hvc1.…/hev1.…). */
  readonly codec: string;
  /** The avcC/hvcC payload, VideoDecoderConfig.description. */
  readonly description: Uint8Array;
  readonly codedWidth: number;
  readonly codedHeight: number;
  /** Byte offset and size of each sample in the file. */
  readonly offsets: Float64Array;
  readonly sizes: Uint32Array;
  /** Presentation time of each sample (seconds, after the edit list). */
  readonly pts: Float64Array;
  /** Each sample's duration (seconds). */
  readonly durations: Float64Array;
  /** 1 for a sync sample (keyframe). */
  readonly key: Uint8Array;
  /** Sample indices sorted by presentation time. */
  readonly order: Uint32Array;
  /** Clockwise degrees the decoded picture is turned for display (the track matrix, as <video> applies it). */
  readonly rotation: 0 | 90 | 180 | 270;
  /** The footage's colour description (H.273 codes; absent fields unspecified), or undefined when its SPS doesn't parse. */
  readonly colour: StreamColour | undefined;
  /** Each sample's position in `order`. */
  readonly rank: Uint32Array;
}

/** Most samples a track may have (10 h at 240 fps); more is "no index". */
const MAX_SAMPLES = 10 * 3600 * 240;
/** Largest moov the walker reads into memory. */
export const MAX_MOOV = 128 * 1024 * 1024;

type Box = { readonly type: string; readonly start: number; readonly end: number };

const u32 = (d: DataView, at: number) => d.getUint32(at);
const fourcc = (d: Uint8Array, at: number) => String.fromCharCode(d[at]!, d[at + 1]!, d[at + 2]!, d[at + 3]!);

/** Child boxes in [start, end) of `d` (stops quietly at a truncated or malformed box). */
function children(d: Uint8Array, start: number, end: number): Array<Box> {
  const view = new DataView(d.buffer, d.byteOffset, d.byteLength);
  const out: Array<Box> = [];
  let i = start;
  while (i + 8 <= end && out.length < 100_000) {
    let size = u32(view, i);
    const type = fourcc(d, i + 4);
    let header = 8;
    if (size === 1) {
      if (i + 16 > end) break;
      size = Number(view.getBigUint64(i + 8));
      header = 16;
    } else if (size === 0) size = end - i;
    if (size < header || i + size > end) break;
    out.push({ type, start: i + header, end: i + size });
    i += size;
  }
  return out;
}

const child = (d: Uint8Array, box: Box, type: string) => children(d, box.start, box.end).find((b) => b.type === type);
const path = (d: Uint8Array, box: Box | undefined, ...types: Array<string>) => {
  let b = box;
  for (const t of types) b = b && child(d, b, t);
  return b;
};

const hex = (n: number) => n.toString(16).padStart(2, "0");

/** The WebCodecs codec string of an avcC record. */
export function avcCodec(avcc: Uint8Array): string | undefined {
  if (avcc.length < 4 || avcc[0] !== 1) return undefined;
  return `avc1.${hex(avcc[1]!)}${hex(avcc[2]!)}${hex(avcc[3]!)}`;
}

/** The WebCodecs codec string of an hvcC record (ISO/IEC 14496-15 annex E), as `prefix` (hvc1/hev1). */
export function hevcCodec(hvcc: Uint8Array, prefix: string): string | undefined {
  if (hvcc.length < 13) return undefined;
  const b1 = hvcc[1]!;
  const space = ["", "A", "B", "C"][b1 >> 6]!;
  const tier = (b1 >> 5) & 1 ? "H" : "L";
  const profile = b1 & 0x1f;
  let compat = ((hvcc[2]! << 24) | (hvcc[3]! << 16) | (hvcc[4]! << 8) | hvcc[5]!) >>> 0;
  // The compatibility flags are written bit-reversed.
  let reversed = 0;
  for (let i = 0; i < 32; i++) {
    reversed = ((reversed << 1) | (compat & 1)) >>> 0;
    compat >>>= 1;
  }
  const constraints = [...hvcc.subarray(6, 12)];
  while (constraints.length && constraints.at(-1) === 0) constraints.pop();
  const level = hvcc[12]!;
  return [`${prefix}.${space}${profile}`, reversed.toString(16).toUpperCase(), `${tier}${level}`, ...constraints.map((c) => c.toString(16).toUpperCase())].join(".");
}

/** Reads `length` bytes at `offset` of the file (a Range fetch in the renderer, a file read in tests). */
export type ReadAt = (offset: number, length: number) => Promise<Uint8Array>;

/**
 * Finds the file's `moov` by walking its top-level boxes (one small read per box header, so a
 * moov after a multi-gigabyte mdat costs a few reads) and indexes its video track.
 */
export async function readMp4Index(read: ReadAt, size: number): Promise<Mp4Index | undefined> {
  let at = 0;
  for (let n = 0; n < 10_000 && at + 8 <= size; n++) {
    const head = await read(at, Math.min(16, size - at));
    if (head.length < 8) return undefined;
    const view = new DataView(head.buffer, head.byteOffset, head.byteLength);
    let boxSize = u32(view, 0);
    const type = fourcc(head, 4);
    let header = 8;
    if (boxSize === 1) {
      if (head.length < 16) return undefined;
      boxSize = Number(view.getBigUint64(8));
      header = 16;
    } else if (boxSize === 0) boxSize = size - at;
    if (boxSize < header || at + boxSize > size) return undefined;
    // A fragmented file: its samples are in moofs, not indexed here.
    if (type === "moof") return undefined;
    if (type === "moov") {
      if (boxSize > MAX_MOOV) return undefined;
      const moov = await read(at, boxSize);
      if (moov.length < boxSize) return undefined;
      return parseMoov(moov.subarray(header));
    }
    at += boxSize;
  }
  return undefined;
}

interface Edits {
  /** Seconds the track is delayed by a leading empty edit. */
  readonly delay: number;
  /** Media time (track timescale) where the shown track starts. */
  readonly mediaTime: number;
}

/** The edit list as one shift, or "unsupported" for several real edits or a rate other than 1. */
function edits(d: Uint8Array, trak: Box, movieScale: number): Edits | "unsupported" {
  const elst = path(d, trak, "edts", "elst");
  if (!elst) return { delay: 0, mediaTime: 0 };
  const view = new DataView(d.buffer, d.byteOffset, d.byteLength);
  if (elst.end - elst.start < 8) return "unsupported";
  const version = d[elst.start]!;
  const count = u32(view, elst.start + 4);
  const entry = version === 1 ? 20 : 12;
  if (count > 1000 || elst.start + 8 + count * entry > elst.end) return "unsupported";
  let delay = 0;
  const real: Array<number> = [];
  for (let i = 0; i < count; i++) {
    const p = elst.start + 8 + i * entry;
    const duration = version === 1 ? Number(view.getBigUint64(p)) : u32(view, p);
    const mediaTime = version === 1 ? Number(view.getBigInt64(p + 8)) : view.getInt32(p + 4);
    const rate = view.getInt16(p + (version === 1 ? 16 : 8));
    if (mediaTime === -1) {
      if (real.length) return "unsupported";
      delay += duration / (movieScale || 1);
      continue;
    }
    if (rate !== 1) return "unsupported";
    real.push(mediaTime);
  }
  if (real.length > 1) return "unsupported";
  return { delay, mediaTime: real[0] ?? 0 };
}

/**
 * The track header's display matrix as a clockwise turn, or undefined for anything else (a flip, a
 * scale, a shear: not drawn the way <video> would, so not indexed).
 */
function rotationOf(d: Uint8Array, trak: Box): 0 | 90 | 180 | 270 | undefined {
  const tkhd = child(d, trak, "tkhd");
  if (!tkhd) return 0;
  const at = tkhd.start + (d[tkhd.start] === 1 ? 52 : 40);
  if (at + 36 > tkhd.end) return 0;
  const view = new DataView(d.buffer, d.byteOffset, d.byteLength);
  // 16.16 fixed point: a b u / c d v / x y w.
  const [a, b, c, dd] = [view.getInt32(at), view.getInt32(at + 4), view.getInt32(at + 12), view.getInt32(at + 16)];
  const one = 0x10000;
  if (a === one && b === 0 && c === 0 && dd === one) return 0;
  if (a === 0 && b === one && c === -one && dd === 0) return 90;
  if (a === -one && b === 0 && c === 0 && dd === -one) return 180;
  if (a === 0 && b === -one && c === one && dd === 0) return 270;
  return undefined;
}

/** A colr box's nclx/nclc colour description (unspecified codes left out). */
function colrOf(d: Uint8Array, box: Box | undefined): StreamColour {
  if (!box || box.end - box.start < 10) return {};
  const kind = fourcc(d, box.start);
  if (kind !== "nclx" && kind !== "nclc") return {};
  const view = new DataView(d.buffer, d.byteOffset, d.byteLength);
  const code = (at: number) => {
    const v = view.getUint16(box.start + at);
    return v === 2 ? undefined : v;
  };
  const full = kind === "nclx" && box.end - box.start >= 11 ? (d[box.start + 10]! & 0x80) !== 0 : undefined;
  return { ...(code(4) !== undefined ? { primaries: code(4)! } : {}), ...(code(6) !== undefined ? { transfer: code(6)! } : {}), ...(code(8) !== undefined ? { matrix: code(8)! } : {}), ...(full !== undefined ? { fullRange: full } : {}) };
}

/** The container's description over the stream's, field by field (unspecified fields left out). */
function mergeColour(colr: StreamColour, sps: StreamColour | undefined): StreamColour | undefined {
  if (!sps) return Object.keys(colr).length ? colr : undefined;
  const pick = (k: "primaries" | "transfer" | "matrix") => colr[k] ?? (sps[k] === 2 ? undefined : sps[k]);
  const out: { primaries?: number; transfer?: number; matrix?: number; fullRange?: boolean } = {};
  for (const k of ["primaries", "transfer", "matrix"] as const) {
    const v = pick(k);
    if (v !== undefined) out[k] = v;
  }
  const full = colr.fullRange ?? sps.fullRange;
  if (full !== undefined) out.fullRange = full;
  return out;
}

interface Candidate {
  readonly trak: Box;
  readonly colour: StreamColour | undefined;
  readonly codec: string;
  readonly description: Uint8Array;
  readonly width: number;
  readonly height: number;
}

/** The track's video sample entry, if it is H.264 or HEVC. */
function videoEntry(d: Uint8Array, trak: Box): Candidate | undefined {
  const hdlr = path(d, trak, "mdia", "hdlr");
  if (!hdlr || hdlr.end - hdlr.start < 12 || fourcc(d, hdlr.start + 8) !== "vide") return undefined;
  const stsd = path(d, trak, "mdia", "minf", "stbl", "stsd");
  if (!stsd || stsd.end - stsd.start < 8) return undefined;
  const view = new DataView(d.buffer, d.byteOffset, d.byteLength);
  // One sample description: several (a codec change mid-file) isn't decodable with one config.
  if (u32(view, stsd.start + 4) !== 1) return undefined;
  const [entry] = children(d, stsd.start + 8, stsd.end);
  if (!entry || entry.end - entry.start < 78) return undefined;
  const type = entry.type;
  const width = view.getUint16(entry.start + 24);
  const height = view.getUint16(entry.start + 26);
  const boxes = children(d, entry.start + 78, entry.end);
  if (type === "avc1" || type === "avc3") {
    const avcc = boxes.find((b) => b.type === "avcC");
    // avc3 carries its parameter sets in band: still decodable, but not with this description alone.
    if (!avcc || type === "avc3") return undefined;
    const description = d.slice(avcc.start, avcc.end);
    const codec = avcCodec(description);
    const sps = avccSps(description)[0];
    const colour = mergeColour(colrOf(d, boxes.find((b) => b.type === "colr")), sps ? h264Colour(sps) : undefined);
    return codec ? { trak, codec, description, width, height, colour } : undefined;
  }
  if (type === "hvc1" || type === "hev1") {
    const hvcc = boxes.find((b) => b.type === "hvcC");
    if (!hvcc) return undefined;
    const description = d.slice(hvcc.start, hvcc.end);
    const codec = hevcCodec(description, type);
    const sps = hvccSps(description)[0];
    const colour = mergeColour(colrOf(d, boxes.find((b) => b.type === "colr")), sps ? hevcColour(sps) : undefined);
    return codec ? { trak, codec, description, width, height, colour } : undefined;
  }
  return undefined;
}

/** A full box's entry table: its count and where the entries start, or undefined if it doesn't fit. */
function table(d: Uint8Array, box: Box | undefined, entrySize: number, countAt = 4): { count: number; at: number } | undefined {
  if (!box || box.end - box.start < countAt + 4) return undefined;
  const view = new DataView(d.buffer, d.byteOffset, d.byteLength);
  const count = u32(view, box.start + countAt);
  const at = box.start + countAt + 4;
  if (count > MAX_SAMPLES * 2 || at + count * entrySize > box.end) return undefined;
  return { count, at };
}

/** Indexes the largest H.264/HEVC video track of a `moov` payload (the box's body). */
export function parseMoov(moov: Uint8Array): Mp4Index | undefined {
  const d = moov;
  const view = new DataView(d.buffer, d.byteOffset, d.byteLength);
  const top = children(d, 0, d.length);
  // Fragmented: samples live in moofs.
  if (top.some((b) => b.type === "mvex")) return undefined;
  const mvhd = top.find((b) => b.type === "mvhd");
  const movieScale = mvhd && mvhd.end - mvhd.start >= 24 ? u32(view, mvhd.start + (d[mvhd.start] === 1 ? 20 : 12)) : 0;
  const candidates = top.filter((b) => b.type === "trak").map((t) => videoEntry(d, t)).filter((c): c is Candidate => !!c);
  // The largest video track, as ffmpeg's (and Chromium's) default stream choice.
  const best = candidates.sort((a, b) => b.width * b.height - a.width * a.height)[0];
  if (!best) return undefined;
  const { trak } = best;
  const mdhd = path(d, trak, "mdia", "mdhd");
  if (!mdhd || mdhd.end - mdhd.start < 24) return undefined;
  const timescale = u32(view, mdhd.start + (d[mdhd.start] === 1 ? 20 : 12));
  if (!timescale) return undefined;
  const edit = edits(d, trak, movieScale);
  if (edit === "unsupported") return undefined;
  const rotation = rotationOf(d, trak);
  if (rotation === undefined) return undefined;
  const stbl = path(d, trak, "mdia", "minf", "stbl")!;
  const box = (t: string) => child(d, stbl, t);

  // Sizes.
  const stsz = box("stsz");
  let sizes: Uint32Array;
  if (stsz) {
    if (stsz.end - stsz.start < 12) return undefined;
    const fixed = u32(view, stsz.start + 4);
    const count = u32(view, stsz.start + 8);
    if (!count || count > MAX_SAMPLES) return undefined;
    sizes = new Uint32Array(count);
    if (fixed) sizes.fill(fixed);
    else {
      if (stsz.start + 12 + count * 4 > stsz.end) return undefined;
      for (let i = 0; i < count; i++) sizes[i] = u32(view, stsz.start + 12 + i * 4);
    }
  } else return undefined;
  const count = sizes.length;

  // Chunk offsets, then each sample's offset through sample-to-chunk.
  const stco = box("stco");
  const co64 = box("co64");
  const chunks = co64 ? table(d, co64, 8) : table(d, stco, 4);
  if (!chunks || !chunks.count) return undefined;
  const chunkOffset = (i: number) => (co64 ? Number(view.getBigUint64(chunks.at + i * 8)) : u32(view, chunks.at + i * 4));
  const stsc = table(d, box("stsc"), 12);
  if (!stsc || !stsc.count) return undefined;
  const offsets = new Float64Array(count);
  let sample = 0;
  for (let e = 0; e < stsc.count && sample < count; e++) {
    const first = u32(view, stsc.at + e * 12) - 1;
    const perChunk = u32(view, stsc.at + e * 12 + 4);
    const nextFirst = e + 1 < stsc.count ? u32(view, stsc.at + (e + 1) * 12) - 1 : chunks.count;
    if (first < 0 || nextFirst < first || !perChunk) return undefined;
    for (let c = first; c < Math.min(nextFirst, chunks.count) && sample < count; c++) {
      let at = chunkOffset(c);
      for (let k = 0; k < perChunk && sample < count; k++) {
        offsets[sample] = at;
        at += sizes[sample]!;
        sample++;
      }
    }
  }
  if (sample < count) return undefined;

  // Decode times and durations.
  const stts = table(d, box("stts"), 8);
  if (!stts) return undefined;
  const dts = new Float64Array(count);
  const dur = new Float64Array(count);
  sample = 0;
  let t = 0;
  for (let e = 0; e < stts.count && sample < count; e++) {
    const n = u32(view, stts.at + e * 8);
    const delta = u32(view, stts.at + e * 8 + 4);
    for (let k = 0; k < n && sample < count; k++) {
      dts[sample] = t;
      dur[sample] = delta;
      t += delta;
      sample++;
    }
  }
  if (sample < count) return undefined;

  // Composition offsets (B-frames), signed in version 1 (and read signed in 0 as ffmpeg does).
  const ctts = box("ctts");
  const cts = Float64Array.from(dts);
  const ct = table(d, ctts, 8);
  if (ctts && !ct) return undefined;
  if (ct) {
    sample = 0;
    for (let e = 0; e < ct.count && sample < count; e++) {
      const n = u32(view, ct.at + e * 8);
      const off = view.getInt32(ct.at + e * 8 + 4);
      for (let k = 0; k < n && sample < count; k++) cts[sample++]! += off;
    }
  }

  // Keyframes: every sample when there's no stss.
  const key = new Uint8Array(count);
  const stss = box("stss");
  const ss = table(d, stss, 4);
  if (stss && !ss) return undefined;
  if (ss) {
    for (let e = 0; e < ss.count; e++) {
      const n = u32(view, ss.at + e * 4) - 1;
      if (n >= 0 && n < count) key[n] = 1;
    }
  } else key.fill(1);
  if (!key[0]) return undefined;

  const pts = new Float64Array(count);
  const durations = new Float64Array(count);
  for (let i = 0; i < count; i++) {
    pts[i] = (cts[i]! - edit.mediaTime) / timescale + edit.delay;
    durations[i] = dur[i]! / timescale;
  }
  const order = Uint32Array.from({ length: count }, (_, i) => i).sort((a, b) => pts[a]! - pts[b]! || a - b);
  const rank = new Uint32Array(count);
  for (let i = 0; i < count; i++) rank[order[i]!] = i;
  return { codec: best.codec, description: best.description, codedWidth: best.width, codedHeight: best.height, offsets, sizes, pts, durations, key, order, rank, rotation, colour: best.colour };
}

/**
 * The sample a <video> shows at `t` seconds: the last one presented at or before `t`, or the first
 * shown one (pts ≥ 0) before that. Samples before 0 (cut by the edit list) are never shown.
 */
export function sampleAt(index: Mp4Index, t: number): number {
  const { order, pts } = index;
  let first = 0;
  while (first < order.length - 1 && pts[order[first]!]! < -1e-6) first++;
  let lo = first;
  let hi = order.length - 1;
  if (pts[order[lo]!]! > t) return order[lo]!;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (pts[order[mid]!]! <= t) lo = mid;
    else hi = mid - 1;
  }
  return order[lo]!;
}

/** The keyframe (decode order) a sample's decode starts from. */
export function keyframeOf(index: Mp4Index, sample: number): number {
  let k = sample;
  while (k > 0 && !index.key[k]) k--;
  return k;
}
