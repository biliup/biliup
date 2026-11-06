// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { ProjectSnapshot } from "@/app/lib/editor/contracts";
import { createContext, useContext, useSyncExternalStore } from "react";
import type { AppStateShape, Project } from "@/app/lib/editor/state/store/types";

import { createEditorStore, initialState, type EditorStore } from '@/app/lib/editor/project-store';
export { createEditorStore } from '@/app/lib/editor/project-store';
let activeStore: EditorStore | undefined;
export let state = initialState();
export const EditorStoreContext = createContext<EditorStore | undefined>(undefined);
export function activateEditorStore(store: EditorStore) {
 activeStore = store; state = store.getSnapshot();
 const release = store.subscribe(() => { if (activeStore === store) state = store.getSnapshot(); });
 return () => { release(); if (activeStore === store) { activeStore = undefined; state = initialState(); } };
}
export const set = (patch: Partial<AppStateShape>) => activeStore?.set(patch);
export const currentEditorStore = () => activeStore;
/** Outside React: called after every state change (D144: the recorder watches an agent take). */
export const subscribeApp = (listener: () => void) => activeStore?.subscribe(listener) ?? (() => {});
export const useApp = () => { const store = useContext(EditorStoreContext); if (!store) throw new Error('EditorProvider missing'); return useSyncExternalStore(store.subscribe, store.getSnapshot, store.getSnapshot); };
export const useAppSelect = <T,>(select: (s: AppStateShape) => T): T => { const store = useContext(EditorStoreContext); if (!store) throw new Error('EditorProvider missing'); return useSyncExternalStore(store.subscribe, () => select(store.getSnapshot()), () => select(store.getSnapshot())); };
export const getState = () => state;
export const projectOf = (snapshot: ProjectSnapshot) => snapshot.project as Project;
export const online = () => state.connection === 'connected';
