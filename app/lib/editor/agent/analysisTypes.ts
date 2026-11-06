// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type {StoredEnvelope} from "../core/text/transcript";
export {fillersOf,pausesOf} from "../core/text/transcript";
export type {Filler,Pause} from "../core/text/transcript";
export interface TranscriptWord {
  readonly text: string;
  /** Seconds of the file. */
  readonly start: number;
  readonly end: number;
  /** whisper's confidence, 0–1 (the lowest of the word's pieces). */
  readonly p: number;
}
export interface TranscriptSegment {
  readonly start: number;
  readonly end: number;
  readonly text: string;
  /** Its words: indices [first, end) into words. */
  readonly words: readonly [number, number];
}
export interface TranscriptResult {
  readonly hasAudio: boolean;
  readonly seconds?: number;
  /** The spoken language (detected, or the one asked for), ISO 639-1. */
  readonly language?: string;
  /** What was asked: "auto" or a language code. */
  readonly requestedLanguage?: string;
  /** The model that made it (a whisperModels id). */
  readonly model?: string;
  readonly segments?: ReadonlyArray<TranscriptSegment>;
  readonly words?: ReadonlyArray<TranscriptWord>;
}
export interface Onset {
  /** Seconds of the file. */
  readonly t: number;
  /** 0–1, relative to the file's strong onsets. */
  readonly strength: number;
}
export interface BeatsResult {
  readonly hasAudio: boolean;
  readonly seconds?: number;
  /** Beats per minute, or null when there's no steady beat (speech, silence). */
  readonly bpm?: number | null;
  /** How clear the pulse is, 0–1. */
  readonly confidence?: number;
  /** Beat times (seconds), in order. */
  readonly beats?: ReadonlyArray<number>;
  readonly onsets?: ReadonlyArray<Onset>;
  /** D296: the low bands' level from each beat to the next, relative to the loudest (0–1), one per beat. */
  readonly beatLevels?: ReadonlyArray<number>;
}
export interface SilenceRange {
  readonly start: number;
  readonly end: number;
}
export interface LoudnessResult {
  readonly hasAudio: boolean;
  /** Seconds of sound decoded. */
  readonly seconds?: number;
  /** EBU R128 integrated loudness (LUFS); null when too short or silent to measure. */
  readonly integratedLufs?: number | null;
  /** Loudness range (LU). */
  readonly loudnessRangeLu?: number | null;
  /** True peak (dBTP). */
  readonly truePeakDbtp?: number | null;
  readonly silence?: { readonly thresholdDb: number; readonly minSeconds: number; readonly ranges: ReadonlyArray<SilenceRange> };
  /** The speech envelope (D74, version 2): RMS dBFS every 10 ms, quantised (core decodeEnvelope). */
  readonly envelope?: StoredEnvelope;
}
