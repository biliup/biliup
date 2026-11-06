// Adapted from Uncut agentShaders.ts, MIT; licenses/uncut/LICENSE.
export function samplePicture(w: number, h: number): Uint8Array<ArrayBuffer> {
  const px = new Uint8Array(w * h * 4);
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++) {
      const u = x / w;
      const v = y / h;
      let r = 60 + 180 * v;
      let g = 110 + 80 * v;
      let b = 190 - 100 * v;
      if (Math.hypot(u - 0.65, v - 0.45) < 0.08) [r, g, b] = [255, 244, 196];
      if (v > 0.62 + 0.06 * Math.sin(u * 9)) [r, g, b] = [59, 93, 58];
      if (v > 0.72) [r, g, b] = [30 + 30 * (1 - v), 90 - 40 * v, 120 - 50 * v];
      if (v > 0.82 && v < 0.94 && u > 0.4) {
        const swatch = [[230, 57, 70], [244, 211, 94], [42, 157, 143], [91, 75, 219], [255, 255, 255], [17, 17, 17]][Math.min(5, Math.floor((u - 0.4) / 0.1))]!;
        [r, g, b] = swatch as [number, number, number];
      }
      const i = (y * w + x) * 4;
      px[i] = r;
      px[i + 1] = g;
      px[i + 2] = b;
      px[i + 3] = 255;
    }
  return px;
}
