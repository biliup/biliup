// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { isTransportError } from "@/app/lib/editor/state/connection";
import { playback } from "@/app/ui/editor/preview/playback";
import { currentEditorStore, set, state } from "@/app/lib/editor/state/store/core";
import { projectEditorHref } from "@/app/lib/workbench";
import { confirmLeave, queuedProjectChange } from "@/app/lib/editor/state/store/edits";
import { call, plainError, refreshRecents, chooseFiles } from "@/app/lib/editor/state/store/server";
import type { EditorDocument } from '@/app/lib/editor/project';

/** When a project last closed (D243). */
export const projectClose = { at: 0 };

/** How long after a project closes a ⌘W (or the window's close) is kept from closing the window (D243). */
const CLOSE_GUARD_MS = 1000;

/** A project is closing or has just closed: a second ⌘W mustn't take the window, and the app, with it (D243). */
export const justClosedProject = () => Date.now() - projectClose.at < CLOSE_GUARD_MS;

export const projectActions = {
  /** Project selection belongs to biliup's application navigation. */
  async openProject(dir: string) {
    const owner = currentEditorStore(), origin = state.snapshot?.dir;
    const id = Number(/^biliup:(\d+)$/.exec(dir)?.[1]);
    if (!Number.isSafeInteger(id) || id <= 0) return;
    if (state.snapshot && state.snapshot.dir !== dir && !(await confirmLeave("open-project"))) return;
    if (currentEditorStore() !== owner || state.snapshot?.dir !== origin) return;
    window.history.pushState(null, '', projectEditorHref(id));
  },

  async openWithDialog() {
    const owner = currentEditorStore(), origin = state.snapshot?.dir;
    if (state.snapshot && !(await confirmLeave("open-project"))) return;
    if (currentEditorStore() !== owner || state.snapshot?.dir !== origin) return;
    window.history.pushState(null, '', '/workbench/projects');
  },

  /**
   * Back to Welcome (D243): the app stays open. While it's under way and just after, a ⌘W (held,
   * or pressed again) is kept from closing the window (keys.ts).
   */
  async closeProject() {
    const owner = currentEditorStore(), origin = state.snapshot?.dir;
    if (!state.snapshot) return;
    if (!(await confirmLeave("close-project"))) return;
    if (currentEditorStore() !== owner || state.snapshot?.dir !== origin) return;
    projectClose.at = Date.now();
    const result = await call("project.close", {});
    if (currentEditorStore() !== owner || state.snapshot?.dir !== origin) return;
    if (!result.ok) return set({ notice: { text: plainError(result.error), transport: isTransportError(result.error) } });
    projectClose.at = Date.now();
    set({ snapshot: null, notice: undefined, welcomeError: undefined });
    playback.reset();
    await refreshRecents();
  },

  undo: () => queuedProjectChange(() => state.snapshot?.canUndo ? call("project.undo", {}) : undefined),
  redo: () => queuedProjectChange(() => state.snapshot?.canRedo ? call("project.redo", {}) : undefined),
};
export const projectFileActions = {
  /** The project as it was at a revision (History's preview), or a sentence saying why not. */
  async historyPreview(revision: number): Promise<{ readonly document: EditorDocument } | { readonly problem: string }> {
    const result = await call("history.preview", { revision });
    return result.ok ? { document: result.value.document } : { problem: plainError(result.error) };
  },
  /** Bring back a revision's state as a new revision (History's Restore; one ⌘Z takes it back). */
  historyJump: (revision: number, origin: { dir: string; baseRevision: number }) =>
    queuedProjectChange(() => call("history.jump", { revision, baseRevision: origin.baseRevision }), { inline: true, origin }),
  async historyMore():Promise<string|undefined>{
    const result=await call("history.more",{});
    return result.ok?undefined:plainError(result.error);
  },

  /** Browser files are uploaded and imported by the owning project session. */
  importMedia: () => chooseFiles(),
  dismissImportReport: () => set({ importReport: undefined }),

  dismissNotice: () => set({ notice: undefined }),
};
