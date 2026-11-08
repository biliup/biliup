// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// D221: the colour description in an H.264 or HEVC sequence parameter set's VUI (video signal type).
// WebCodecs labels a frame without one as BT.709, while Chromium's <video> and ffmpeg (so the export)
// read such footage as BT.601: the scrub cache needs to know which footage says nothing, to decode it
// the way the <video> shows it. Only the fields up to the video signal type are walked; anything that
// doesn't parse is "unknown" (undefined), and the caller leaves WebCodecs' own reading alone.

/** H.273 codes as written in the stream (2 = unspecified). */
export interface StreamColour {
  readonly primaries?: number;
  readonly transfer?: number;
  readonly matrix?: number;
  readonly fullRange?: boolean;
}

/** A NAL unit's RBSP: emulation-prevention bytes (00 00 03) removed. */
export function rbsp(nal: Uint8Array): Uint8Array {
  const out = new Uint8Array(nal.length);
  let n = 0;
  let zeros = 0;
  for (let i = 0; i < nal.length; i++) {
    const b = nal[i]!;
    if (zeros >= 2 && b === 3) {
      zeros = 0;
      continue;
    }
    zeros = b === 0 ? zeros + 1 : 0;
    out[n++] = b;
  }
  return out.subarray(0, n);
}

class Bits {
  private pos = 0;
  private readonly d: Uint8Array;
  constructor(d: Uint8Array) {
    this.d = d;
  }
  bit(): number {
    if (this.pos >= this.d.length * 8) throw new RangeError("end of the parameter set");
    const v = (this.d[this.pos >> 3]! >> (7 - (this.pos & 7))) & 1;
    this.pos++;
    return v;
  }
  u(n: number): number {
    let v = 0;
    for (let i = 0; i < n; i++) v = v * 2 + this.bit();
    return v;
  }
  skip(n: number) {
    for (let i = 0; i < n; i++) this.bit();
  }
  ue(): number {
    let zeros = 0;
    while (this.bit() === 0) if (++zeros > 31) throw new RangeError("bad exp-Golomb code");
    return (2 ** zeros - 1) + this.u(zeros);
  }
  se(): number {
    const k = this.ue();
    return k & 1 ? (k + 1) / 2 : -k / 2;
  }
}

/** The VUI's video signal type, after aspect ratio and overscan (shared by H.264 and HEVC). */
function videoSignal(b: Bits): StreamColour {
  if (b.bit()) {
    if (b.u(8) === 255) b.skip(32);
  }
  if (b.bit()) b.skip(1);
  if (!b.bit()) return {};
  b.skip(3); // video_format
  const fullRange = b.bit() === 1;
  if (!b.bit()) return { fullRange };
  return { primaries: b.u(8), transfer: b.u(8), matrix: b.u(8), fullRange };
}

/** An H.264 SPS (the NAL unit with its header byte) to its colour description; {} when it has none, undefined when it doesn't parse. */
export function h264Colour(sps: Uint8Array): StreamColour | undefined {
  try {
    const b = new Bits(rbsp(sps.subarray(1)));
    const profile = b.u(8);
    b.skip(16); // constraint flags, level
    b.ue(); // seq_parameter_set_id
    if ([100, 110, 122, 244, 44, 83, 86, 118, 128, 138, 139, 134, 135].includes(profile)) {
      const chroma = b.ue();
      if (chroma === 3) b.skip(1);
      b.ue();
      b.ue();
      b.skip(1);
      if (b.bit()) {
        for (let i = 0; i < (chroma !== 3 ? 8 : 12); i++) {
          if (!b.bit()) continue;
          const size = i < 6 ? 16 : 64;
          let last = 8;
          let next = 8;
          for (let j = 0; j < size && next !== 0; j++) {
            next = (last + b.se() + 256) % 256;
            if (next !== 0) last = next;
          }
        }
      }
    }
    b.ue(); // log2_max_frame_num_minus4
    const poc = b.ue();
    if (poc === 0) b.ue();
    else if (poc === 1) {
      b.skip(1);
      b.se();
      b.se();
      const n = b.ue();
      for (let i = 0; i < n; i++) b.se();
    }
    b.ue(); // max_num_ref_frames
    b.skip(1);
    b.ue();
    b.ue();
    if (!b.bit()) b.skip(1); // frame_mbs_only, mb_adaptive
    b.skip(1); // direct_8x8
    if (b.bit()) for (let i = 0; i < 4; i++) b.ue();
    if (!b.bit()) return {};
    return videoSignal(b);
  } catch {
    return undefined;
  }
}

/** profile_tier_level() of an HEVC VPS/SPS. */
function profileTierLevel(b: Bits, maxSubLayersMinus1: number) {
  b.skip(88 + 8); // general profile (88 bits) and level
  const profilePresent: Array<number> = [];
  const levelPresent: Array<number> = [];
  for (let i = 0; i < maxSubLayersMinus1; i++) {
    profilePresent.push(b.bit());
    levelPresent.push(b.bit());
  }
  if (maxSubLayersMinus1 > 0) for (let i = maxSubLayersMinus1; i < 8; i++) b.skip(2);
  for (let i = 0; i < maxSubLayersMinus1; i++) {
    if (profilePresent[i]) b.skip(88);
    if (levelPresent[i]) b.skip(8);
  }
}

/** HEVC scaling_list_data(). */
function scalingListData(b: Bits) {
  for (let size = 0; size < 4; size++) {
    for (let matrix = 0; matrix < 6; matrix += size === 3 ? 3 : 1) {
      if (!b.bit()) b.ue();
      else {
        const coefs = Math.min(64, 1 << (4 + (size << 1)));
        if (size > 1) b.se();
        for (let i = 0; i < coefs; i++) b.se();
      }
    }
  }
}

/** HEVC st_ref_pic_set(idx); returns its NumDeltaPocs (needed by later sets that predict from it). */
function stRefPicSet(b: Bits, idx: number, numDeltaPocs: Array<number>): number {
  if (idx !== 0 && b.bit()) {
    // inter_ref_pic_set_prediction (delta_idx is only sent in slice headers, never in the SPS).
    b.skip(1);
    b.ue();
    const ref = numDeltaPocs[idx - 1]!;
    let n = 0;
    for (let j = 0; j <= ref; j++) {
      const used = b.bit();
      const useDelta = used ? 1 : b.bit();
      if (used || useDelta) n++;
    }
    return n;
  }
  const neg = b.ue();
  const pos = b.ue();
  if (neg > 64 || pos > 64) throw new RangeError("bad ref pic set");
  for (let i = 0; i < neg + pos; i++) {
    b.ue();
    b.skip(1);
  }
  return neg + pos;
}

/** An HEVC SPS (the NAL unit with its 2-byte header) to its colour description; {} when it has none, undefined when it doesn't parse. */
export function hevcColour(sps: Uint8Array): StreamColour | undefined {
  try {
    const b = new Bits(rbsp(sps.subarray(2)));
    b.skip(4);
    const maxSub = b.u(3);
    b.skip(1);
    profileTierLevel(b, maxSub);
    b.ue(); // sps_seq_parameter_set_id
    const chroma = b.ue();
    if (chroma === 3) b.skip(1);
    b.ue();
    b.ue();
    if (b.bit()) for (let i = 0; i < 4; i++) b.ue();
    b.ue();
    b.ue();
    const pocBits = b.ue() + 4; // log2_max_pic_order_cnt_lsb
    if (pocBits > 16) return undefined;
    const ordering = b.bit();
    for (let i = ordering ? 0 : maxSub; i <= maxSub; i++) {
      b.ue();
      b.ue();
      b.ue();
    }
    for (let i = 0; i < 6; i++) b.ue(); // block sizes, transform hierarchy depths
    if (b.bit() && b.bit()) scalingListData(b);
    b.skip(2); // amp, sao
    if (b.bit()) {
      b.skip(8);
      b.ue();
      b.ue();
      b.skip(1);
    }
    const sets = b.ue();
    if (sets > 64) return undefined;
    const numDeltaPocs: Array<number> = [];
    for (let i = 0; i < sets; i++) numDeltaPocs.push(stRefPicSet(b, i, numDeltaPocs));
    if (b.bit()) {
      const lt = b.ue();
      if (lt > 32) return undefined;
      for (let i = 0; i < lt; i++) b.skip(pocBits + 1);
    }
    b.skip(2); // temporal mvp, strong intra smoothing
    if (!b.bit()) return {};
    return videoSignal(b);
  } catch {
    return undefined;
  }
}

/** The SPS NAL units in an avcC record. */
export function avccSps(avcc: Uint8Array): Array<Uint8Array> {
  const out: Array<Uint8Array> = [];
  if (avcc.length < 6) return out;
  const count = avcc[5]! & 0x1f;
  let at = 6;
  for (let i = 0; i < count && at + 2 <= avcc.length; i++) {
    const len = (avcc[at]! << 8) | avcc[at + 1]!;
    out.push(avcc.subarray(at + 2, at + 2 + len));
    at += 2 + len;
  }
  return out;
}

/** The SPS NAL units (type 33) in an hvcC record. */
export function hvccSps(hvcc: Uint8Array): Array<Uint8Array> {
  const out: Array<Uint8Array> = [];
  if (hvcc.length < 23) return out;
  const arrays = hvcc[22]!;
  let at = 23;
  for (let a = 0; a < arrays && at + 3 <= hvcc.length; a++) {
    const type = hvcc[at]! & 0x3f;
    const n = (hvcc[at + 1]! << 8) | hvcc[at + 2]!;
    at += 3;
    for (let i = 0; i < n && at + 2 <= hvcc.length; i++) {
      const len = (hvcc[at]! << 8) | hvcc[at + 1]!;
      if (type === 33) out.push(hvcc.subarray(at + 2, at + 2 + len));
      at += 2 + len;
    }
  }
  return out;
}
