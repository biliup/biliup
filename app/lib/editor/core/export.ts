// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Export settings shared by the server (which renders) and the renderer (whose dialog shows them).

export type ExportQuality = "draft" | "standard" | "high";
/** F4 (D171): every preset is H.264; HEVC only when chosen (Advanced). */
export type ExportCodec = "h264" | "hevc";

const even = (n: number) => Math.max(2, Math.round(n / 2) * 2);

/**
 * The export size for a quality: Draft 720p, Standard 1080p (by the shorter side, never upscaled),
 * High = the canvas. `shortSide` (Advanced, D171) replaces the preset's, never upscaled either.
 */
export function outputSize(canvas: { readonly width: number; readonly height: number }, quality: ExportQuality, shortSide?: number): { width: number; height: number } {
  const short = Math.min(canvas.width, canvas.height);
  const target = shortSide !== undefined && shortSide > 0 ? shortSide : quality === "draft" ? 720 : quality === "standard" ? 1080 : short;
  const scale = Math.min(1, target / short);
  return { width: even(canvas.width * scale), height: even(canvas.height * scale) };
}

/**
 * Draft and Standard prefer a working server hardware encoder; High defaults to libx264/libx265.
 * Device selection and fallback are handled by biliup's shared encoding service.
 */
export type ExportEncoder = "hardware" | "software";

/** The encoder a preset uses unless Advanced picks one (D212). */
export const defaultEncoder = (quality: ExportQuality): ExportEncoder => (quality === "high" ? "software" : "hardware");

/** libx264's constant rate factor per preset (D171). */
export const SOFTWARE_H264_CRF: Readonly<Record<ExportQuality, number>> = { draft: 26, standard: 20, high: 17 };

export const CODEC_NAMES: Readonly<Record<ExportCodec, string>> = { h264: "H.264", hevc: "HEVC" };

/** Bits per pixel per frame of each preset: HEVC's target bitrate (H.264 is constant quality, D212, unless a bitrate is set). */
const BPP: Readonly<Record<ExportCodec, Readonly<Record<ExportQuality, number>>>> = {
  h264: { draft: 0.06, standard: 0.1, high: 0.16 },
  hevc: { draft: 0.04, standard: 0.06, high: 0.1 },
};

/** The preset's target bitrate in Mb/s (one decimal, at least 1) for a size and frame rate. */
export function presetBitrateMbps(quality: ExportQuality, codec: ExportCodec, size: { readonly width: number; readonly height: number }, fps: number): number {
  return Math.max(1, Math.round((size.width * size.height * fps * BPP[codec][quality]) / 1e5) / 10);
}

/** The shorter sides the Advanced row offers under the canvas's own (never upscaled). */
export function exportShortSides(canvas: { readonly width: number; readonly height: number }): ReadonlyArray<number> {
  const short = Math.min(canvas.width, canvas.height);
  return [short, ...[2160, 1440, 1080, 720, 480].filter((s) => s < short)];
}

/** The frame rates the Advanced row offers: the project's, then the usual lower ones. */
export function exportFrameRates(fps: number): ReadonlyArray<number> {
  return [fps, ...[60, 50, 30, 25, 24].filter((r) => r < fps - 0.01)];
}
