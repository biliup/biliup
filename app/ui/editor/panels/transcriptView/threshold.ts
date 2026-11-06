// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { DEFAULT_PAUSE_MIN_S } from "@/app/lib/editor/core/text/transcript";
import { useSyncExternalStore } from "react";

/** Pause thresholds on offer (seconds); the default is core's DEFAULT_PAUSE_MIN_S. */
export const PAUSE_STEPS = [0.3, 0.5, 0.8, 1, 1.5, 2];
export const PAUSE_SHOWN_S = 0.3;
export let pauseMin = DEFAULT_PAUSE_MIN_S;

// The pause threshold is kept for the session (module state), so switching tabs doesn't reset it.
const thresholdListeners = new Set<() => void>();
const subscribeThreshold = (l: () => void) => (thresholdListeners.add(l), () => void thresholdListeners.delete(l));
export function useThreshold(): [number, (v: number) => void] {
  const value = useSyncExternalStore(subscribeThreshold, () => pauseMin);
  return [value, (v: number) => ((pauseMin = v), thresholdListeners.forEach((l) => l()))];
}
