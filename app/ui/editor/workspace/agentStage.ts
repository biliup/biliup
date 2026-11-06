// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// #75 (D255): a new, empty project shows the agent in the middle of the window ("centre stage").
// It docks into the side panel's Agent tab when the first turn runs or media lands, or when the user
// goes to the editor (Esc, "Go to editor", a click outside it). Going there is remembered per project
// in this browser; the Agent tab's empty state brings it back while the project is still empty.
import { useSyncExternalStore } from "react";
import { browserPreference, useBrowserPreference } from '@/app/lib/editor/browser-preference';

import type { AgentState } from "@/app/lib/editor/contracts";
import type { Project } from "@/app/lib/editor/state/store";

const MAX_REMEMBERED = 50;
const dismissed = browserPreference<ReadonlyArray<string>>('stage.dismissed', [],
  (value): value is ReadonlyArray<string> => Array.isArray(value) && value.length <= MAX_REMEMBERED && value.every(id => typeof id === 'string'));

/** Nothing imported and nothing on any track. */
export const projectEmpty = (project: Project) => project.media.length === 0 && project.tracks.every((t) => t.items.length === 0);

/** The agent has been used in this project (a turn, or one running). Unknown (not loaded yet) counts as used. */
export const agentUsed = (agent: AgentState | null, dir: string) => !agent || agent.dir !== dir || agent.running !== null || agent.turns.length > 0;

/** Whether the stage shows: an empty project whose agent hasn't run, not sent to the editor. */
export const stageWanted = (project: Project, dir: string, agent: AgentState | null, list: ReadonlyArray<string>) =>
  agent?.configured === true && projectEmpty(project) && !agentUsed(agent, dir) && !list.includes(dir);

/** How the stage was sent to the editor. */
export type StageDismissal = "esc" | "link" | "outside";

// Docking (the stage went to the Agent tab): for a moment the side panel's agent slides in from
// the middle (a CSS animation; nothing covers the page, so clicks are never held up).
let docking: string | undefined;
let dockTimer: ReturnType<typeof setTimeout> | undefined;
const dockListeners = new Set<() => void>();
export function markDocked(dir: string) {
  docking = dir;
  dockListeners.forEach((l) => l());
  clearTimeout(dockTimer);
  dockTimer = setTimeout(() => {
    docking = undefined;
    dockListeners.forEach((l) => l());
  }, 700);
}
export const useDocking = (dir: string) =>
  useSyncExternalStore(
    (l) => (dockListeners.add(l), () => void dockListeners.delete(l)),
    () => docking === dir,
    () => false,
  );

export const stage = {
  /** Sends the stage to the editor for this project (remembered). */
  dismiss(dir: string) {
    const previous = dismissed.get();
    if (previous.includes(dir)) return;
    dismissed.set([...previous, dir].slice(-MAX_REMEMBERED));
  },
  /** Brings it back (the Agent tab's empty state). */
  reopen(dir: string) {
    dismissed.set(dismissed.get().filter(d => d !== dir));
  },
};

export const useStageDismissed = () => useBrowserPreference(dismissed);
