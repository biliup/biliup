import { editorText } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Timeline tools (issue #57, D230): what a press on the lanes does. Select (V) is the usual
// select/move/trim; Blade (C) cuts the item under the pointer where it's pressed (snapped like a
// move); Hand (H) drags the timeline's view. A tool key pressed and let go picks that tool; held
// down (longer than HOLD_MS, or used while down) it picks it only until it's let go, then the
// tool before it comes back. Not remembered between launches: the app always opens on Select.
import { useSyncExternalStore } from "react";

import { keysOf, type ActionId } from "@/app/ui/editor/workspace/keymap";

export type Tool = "select" | "blade" | "hand";

// Their keys are the keymap's (#78, D257: V, C, H by default; a preset or the user can change them).
export const TOOLS: ReadonlyArray<{ readonly id: Tool; readonly action: ActionId; readonly label: string }> = [
  { id: "select", action: "toolSelect", get label() { return editorText("Select: click to select, drag to move or trim"); } },
  { id: "blade", action: "toolBlade", get label() { return editorText("Blade: click an item to cut it there"); } },
  { id: "hand", action: "toolHand", get label() { return editorText("Hand: drag to scroll the timeline"); } },
];

/** The tool a single key (e.key, any case) picks under the current keymap, if any. */
export const toolForKey = (key: string): Tool | undefined => TOOLS.find((t) => keysOf(t.action).includes(key.toUpperCase()))?.id;

let tool: Tool = "select";
const listeners = new Set<() => void>();
export const getTool = (): Tool => tool;
export function setTool(next: Tool) {
  if (next === tool) return;
  tool = next;
  for (const l of listeners) l();
}
const subscribe = (l: () => void) => (listeners.add(l), () => void listeners.delete(l));
export const useTool = (): Tool => useSyncExternalStore(subscribe, getTool, getTool);

/** A key held at least this long switches only while it's down. */
export const HOLD_MS = 250;

export interface ToolKeys {
  /** A tool key went down (`repeat`: the key's auto-repeat, ignored). */
  down(next: Tool, key: string, repeat?: boolean): void;
  /** A key went up: a held tool key gives the tool back. */
  up(key: string): void;
  /** A press (pointer) while a tool key is down: the key was held to use the tool, so it goes back on release. */
  pointer(): void;
  /** The window lost focus: a tool key that's down won't report its release, so it's let go now. */
  blur(): void;
}

/** The tap-to-pick, hold-to-borrow rule, on any tool store (tested with a fake clock). */
export function makeToolKeys(get: () => Tool, set: (t: Tool) => void, now: () => number = () => performance.now()): ToolKeys {
  let held: { key: string; from: Tool; at: number; used: boolean } | undefined;
  return {
    down(next, key, repeat = false) {
      if (repeat) return;
      const k = key.toLowerCase();
      // Another tool key while one is held: the first one's tool still comes back at the end.
      held = { key: k, from: held ? held.from : get(), at: now(), used: !!held };
      set(next);
    },
    up(key) {
      if (!held || held.key !== key.toLowerCase()) return;
      const { from, at, used } = held;
      held = undefined;
      if (used || now() - at >= HOLD_MS) set(from);
    },
    pointer() {
      if (held) held.used = true;
    },
    blur() {
      if (!held) return;
      const { from } = held;
      held = undefined;
      set(from);
    },
  };
}

/** The app's tool keys (editor/keys.ts feeds them). */
export const toolKeys = makeToolKeys(getTool, setTool);
