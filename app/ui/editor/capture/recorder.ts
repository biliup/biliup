// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Portable browser capture: native permission prompts, MediaRecorder, managed uploads and
// shared single/multi-source timeline placement. Recording may follow the timeline playhead.
import type { MultiLayout } from "@/app/lib/editor/contracts";

export { PEAK_HOLD_MS, levelDb, MIN_TAKE_MS, useRecorder, getRecorder } from "@/app/ui/editor/capture/recorder/state";
export type { TakeMode, PartKind, Resolution, Input, RecorderState } from "@/app/ui/editor/capture/recorder/state";
export { refreshAccess, loadInputs, openMonitor, closeMonitor, openPreview, closePreview } from "@/app/ui/editor/capture/recorder/devices";
export { NO_SYSTEM_SOUND } from "@/app/ui/editor/capture/recorder/streams";
export { isMulti, startTake, stopTake, pauseTake, resumeTake, takeSeconds, cancelTake, clock, closeAll } from "@/app/ui/editor/capture/recorder/take";
export type { TakeOptions } from "@/app/ui/editor/capture/recorder/take";
export type { MultiLayout };
