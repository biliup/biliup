// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Which Inspector sections are open (F17, D160), remembered on this Mac (localStorage, like the
// Elements grid's hidden list): one flag per section name, shared by every item that has it.
// Sections the user hasn't touched use their default: the rarely used ones (Adjust, Advanced)
// start closed.
import { useSyncExternalStore } from "react";

const KEY = "biliup.inspector.sections";
export const CLOSED_BY_DEFAULT: ReadonlyArray<string> = ["adjust", "advanced"];

const read = (): Readonly<Record<string, boolean>> => {
  try {
    const raw = typeof localStorage === "undefined" ? null : localStorage.getItem(KEY);
    const saved = raw === null ? {} : (JSON.parse(raw) as unknown);
    return saved && typeof saved === "object" && !Array.isArray(saved) ? Object.fromEntries(Object.entries(saved).filter(([, v]) => typeof v === "boolean")) : {};
  } catch {
    return {};
  }
};

let state = read();
const listeners = new Set<() => void>();

export const sectionOpen = (name: string) => state[name] ?? !CLOSED_BY_DEFAULT.includes(name);
export const useSectionOpen = (name: string) => useSyncExternalStore((l) => (listeners.add(l), () => void listeners.delete(l)), () => sectionOpen(name));
export function setSectionOpen(name: string, open: boolean) {
  state = { ...state, [name]: open };
  try {
    localStorage.setItem(KEY, JSON.stringify(state));
  } catch {}
  for (const l of listeners) l();
}
