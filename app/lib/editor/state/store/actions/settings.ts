// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { set, state, currentEditorStore } from "@/app/lib/editor/state/store/core";
import { call, plainError } from "@/app/lib/editor/state/store/server";
import type { SettingsSection } from "@/app/lib/editor/state/store/types";
import { actions } from "@/app/lib/editor/state/store/actions";

const setupRequests=new WeakMap<object,symbol>();

export const settingsActions = {
  /** Settings (G8): Project when a project is open, else App. */
  openSettings(section?: SettingsSection, focus?: "keys" | "transcription") {
    set({ settingsOpen: true, shortcutsOpen: false, settingsFocus: focus, settingsSection: section ?? (state.snapshot ? state.settingsSection : state.settingsSection === "project" ? "app" : state.settingsSection) });
  },
  closeSettings: () => set({ settingsOpen: false, settingsFocus: undefined }),
  openShortcuts: () => set({ shortcutsOpen: true }),
  closeShortcuts: () => set({ shortcutsOpen: false }),
  openSetup() {
    actions.openSettings("services", "transcription");
  },
  async checkSetup() {
    const owner=currentEditorStore();
    if(!owner)return;
    const request=Symbol();
    setupRequests.set(owner,request);
    set({ setupChecking: true, setupError: undefined });
    const result = await call("setup.check", {});
    if(currentEditorStore()!==owner||setupRequests.get(owner)!==request)return;
    setupRequests.delete(owner);
    if (result.ok) set({ setupChecking: false, setupChecks: result.value });
    else set({ setupChecking: false, setupError: plainError(result.error) });
  },
};
