// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type ExportCodec, type ExportEncoder, defaultEncoder, presetBitrateMbps, SOFTWARE_H264_CRF } from "@/app/lib/editor/core";
import type { Quality } from "./types.ts";

// ---- ffmpeg ----

/**
 * What the rasteriser made for a raster layer: one PNG, a numbered sequence (one file per frame,
 * hard links to its few looks), or a streamed layer's one lossless video (D291: FFV1, straight
 * BGRA, alpha exact) of `frames` frames.
 */
export type RasterOutput =
  | { readonly kind: "still"; readonly png: string }
  | { readonly kind: "sequence"; readonly pattern: string }
  | { readonly kind: "video"; readonly file: string; readonly frames: number };

export interface Caps {
  readonly encoders: ReadonlySet<string>;
  /** ffmpeg's major version (undefined for builds that don't say, e.g. git snapshots). */
  readonly major?: number | undefined;
}

/** How to hand ffmpeg a filter graph in a file: `-/filter_complex` from 7 on, `-filter_complex_script` before (deprecated since 7). */
export const graphFileArgs = (caps: Pick<Caps, "major">, file: string) => (caps.major !== undefined && caps.major < 7 ? ["-filter_complex_script", file] : ["-/filter_complex", file]);

/** F4 (D171): what the Advanced row changes; absent = the preset's (H.264, its encoder and rate). */
export interface VideoChoice {
  readonly codec?: ExportCodec | undefined;
  /** D212: hardware or software; absent = the preset's (`defaultEncoder`). */
  readonly encoder?: ExportEncoder | undefined;
  readonly bitrateMbps?: number | undefined;
}


/** Portable encoder selection. Device probing/fallback belongs to the Rust job runner. */
const hardwareNames = (codec: ExportCodec) => ["nvenc", "qsv", "vaapi", "amf"].map(backend => `${codec}_${backend}`);
export const canHevc = (caps: Pick<Caps,"encoders">) => caps.encoders.has("libx265") || hardwareNames("hevc").some(n => caps.encoders.has(n));
export const usesHardware = (quality: Quality, caps: Pick<Caps,"encoders">, choice: VideoChoice = {}) => (choice.encoder ?? defaultEncoder(quality)) === "hardware" && hardwareNames(choice.codec ?? "h264").some(n => caps.encoders.has(n));
export function videoCodec(quality: Quality, caps: Caps, size: {width:number;height:number}, fps: number, choice: VideoChoice = {}): string[] {
  const codec = choice.codec ?? "h264";
  const mbps = choice.bitrateMbps ?? presetBitrateMbps(quality,codec,size,fps);
  const rate = `${mbps}M`;
  const tags = ["-tag:v",codec === "hevc" ? "hvc1" : "avc1","-bsf:v",codec+"_metadata=colour_primaries=1:transfer_characteristics=1:matrix_coefficients=1:video_full_range_flag=0"];
  if (usesHardware(quality,caps,choice)) {
    const encoder = hardwareNames(codec).find(name=>caps.encoders.has(name))!;
    const mode = encoder.endsWith("_vaapi") ? ["-rc_mode", "VBR"] : encoder.endsWith("_amf") ? ["-rc", "vbr_peak"] : encoder.endsWith("_nvenc") ? ["-rc", "vbr"] : [];
    return ["-c:v",encoder,"-profile:v",codec === "hevc" ? "main" : "high",...mode,"-b:v",rate,"-maxrate",`${mbps * 2}M`,"-bufsize",`${mbps * 4}M`,...tags];
  }
  if (codec === "hevc") return ["-c:v","libx265","-preset",quality === "draft" ? "veryfast" : "medium","-b:v",rate,...tags];
  return ["-c:v","libx264","-preset",quality === "draft" ? "veryfast" : "medium",...(choice.bitrateMbps ? ["-b:v",rate] : ["-crf",String(SOFTWARE_H264_CRF[quality])]),...tags];
}
export const hardwareEncoderFailed = (stderr: string) => /nvenc|qsv|vaapi|amf|CUDA|MFX|Error (?:while opening|initializing output stream)|Could not open encoder|encoder.*(?:not supported|failed)/i.test(stderr);
