// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { useSyncExternalStore } from "react";
import {editorText} from '@/app/lib/editor/i18n';
import type { MediaAccessKind, MediaAccess } from '../access';

export type TakeMode = "voiceover" | "camera" | "screen" | "system" | "multi";
/** D211: a source of a multi-source take, each recorded into its own file. */
export type PartKind = "screen" | "camera" | "voiceover" | "system";
export type Resolution = "720p" | "1080p";

export interface Input {
  readonly id: string;
  readonly label: string;
}

export interface RecorderState {
  readonly access: Readonly<Record<MediaAccessKind, MediaAccess>> | undefined;
  readonly mics: ReadonlyArray<Input>;
  readonly cameras: ReadonlyArray<Input>;
  /** The microphone's level now, 0 (silence, -60 dBFS or less) to 1 (0 dBFS). */
  readonly level: number;
  /** D209: the highest level of the last PEAK_HOLD_MS (the meter's peak mark), same scale. */
  readonly peak: number;
  readonly phase: "idle" | "countdown" | "recording" | "saving";
  /** D209: the running take is paused (MediaRecorder.pause: the gap isn't in the file). */
  readonly paused: boolean;
  /** Milliseconds the take spent paused so far (left out of the timer and recordedMs). */
  readonly pausedMs: number;
  /** When the current pause began (Date.now()). */
  readonly pausedAt: number | undefined;
  /** What the running take records. */
  readonly mode: TakeMode | undefined;
  /** 3, 2, 1 before the take starts. */
  readonly count: number | undefined;
  /** When the recorder started (Date.now()), for the timer. */
  readonly startedAt: number | undefined;
  /** The last take's length as the recorder measured it (start to stop events), ms. */
  readonly recordedMs: number | undefined;
  readonly message: { readonly kind: "saved" | "discarded" | "error"; readonly text: string } | undefined;
  /** The camera's live picture while the Camera part is open. */
  readonly preview: MediaStream | undefined;
  readonly previewError: string | undefined;
  /** D144: the running take answers the agent's screen_record (its card id). */
  readonly agentId: string | undefined;
  /** D211: the running multi-source take's sources, and its screen's live picture (the composite preview). */
  readonly parts: ReadonlyArray<PartKind> | undefined;
  readonly screenLive: MediaStream | undefined;
  /** D211: the last multi-source take's start offsets (ms after the first recorder started), by source. */
  readonly offsets: Readonly<Partial<Record<PartKind, number>>> | undefined;
}

/** How long the meter's peak mark stays before it falls back (D209). */
export const PEAK_HOLD_MS = 1500;
/** The meter's dB scale: -60 dBFS (0) to 0 dBFS (1). */
export const levelDb = (level: number) => (level <= 0 ? -Infinity : level * 60 - 60);

/** Shorter than this, a take is discarded (a click on Record then Stop). */
export const MIN_TAKE_MS = 1000;

export let state: RecorderState = {
  access: undefined,
  mics: [],
  cameras: [],
  level: 0,
  peak: 0,
  phase: "idle",
  paused: false,
  pausedMs: 0,
  pausedAt: undefined,
  mode: undefined,
  count: undefined,
  startedAt: undefined,
  recordedMs: undefined,
  message: undefined,
  preview: undefined,
  previewError: undefined,
  agentId: undefined,
  parts: undefined,
  screenLive: undefined,
  offsets: undefined,
};
export const listeners = new Set<() => void>();
export const set = (patch: Partial<RecorderState>) => {
  state = { ...state, ...patch };
  for (const l of listeners) l();
};
const subscribe = (l: () => void) => (listeners.add(l), () => void listeners.delete(l));
export const useRecorder = <T>(select: (s: RecorderState) => T): T => useSyncExternalStore(subscribe, () => select(state));
export const getRecorder = () => state;

export const stopStream = (stream: MediaStream | undefined) => stream?.getTracks().forEach((t) => t.stop());
export const errorText = (error: unknown) => {
  const e = error as { name?: string; message?: string };
  if (e?.name === "NotAllowedError") return editorText("Recording permission was denied or sharing was cancelled.");
  if (e?.name === "NotFoundError" || e?.name === "OverconstrainedError") return editorText("That device isn't there any more.");
  if (e?.name === "NotReadableError") return editorText("Another app is using it.");
  return e?.message || String(error);
};
