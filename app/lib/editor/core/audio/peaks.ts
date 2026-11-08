/** Maxima of immutable waveform bytes, indexed in small blocks for repeated zoom/redraw queries. */
const BLOCK = 256;
const indexes = new WeakMap<Uint8Array, Uint8Array>();

export function peakBetween(data: Uint8Array, from: number, to: number): number {
  let blocks = indexes.get(data);
  if (!blocks) {
    blocks = new Uint8Array(Math.ceil(data.length / BLOCK));
    for (let i = 0; i < data.length; i++) {
      const block = Math.floor(i / BLOCK);
      blocks[block] = Math.max(blocks[block]!, data[i]!);
    }
    indexes.set(data, blocks);
  }
  let a = Math.max(0, Math.floor(from));
  const b = Math.min(data.length, Math.ceil(to));
  let peak = 0;
  while (a < b && a % BLOCK) peak = Math.max(peak, data[a++]!);
  while (a + BLOCK <= b) {
    peak = Math.max(peak, blocks[Math.floor(a / BLOCK)]!);
    a += BLOCK;
  }
  while (a < b) peak = Math.max(peak, data[a++]!);
  return peak;
}
