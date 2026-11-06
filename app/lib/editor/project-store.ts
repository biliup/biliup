import type { AppStateShape } from './state/store/types'

export const initialState = (): AppStateShape => ({
  ready: false,
  connection: "connecting",
  snapshot: null,
  recents: [],
  settingsOpen: false,
  settingsSection: "project",
  shortcutsOpen: false,
  setupChecking: false,
  jobs: [],
  agent: null,
  agentDraft: "",
  agentMentions: [],
  agentImages: [],
  agentFocus: 0,
  history: null,
  exportOpen: false,
  exportJobs: [],
  analysisJobs: [],
  proxies: [],
  generateJobs: [],
  generateStarting: false,
});
export function createEditorStore() {
 let value = initialState();
 const listeners = new Set<() => void>();
 return {
  getSnapshot: () => value,
  subscribe: (listener: () => void) => { listeners.add(listener); return () => { listeners.delete(listener); }; },
  set(patch: Partial<AppStateShape>) { value = { ...value, ...patch }; listeners.forEach(listener => listener()); },
 };
}
export type EditorStore = ReturnType<typeof createEditorStore>;
