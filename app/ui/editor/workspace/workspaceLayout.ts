// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The editor's panel sizes (G5, D116): the left panel's width, the inspector's width and the
// timeline's height, dragged by thin handles on their edges, remembered in this browser.
// Double-click puts one back to its default. The grid's defaults live in
// styles.css; a remembered size is clamped there too, so a smaller window can't starve the preview.
import { useMemo, useSyncExternalStore } from "react";

export type PanelSize = "lib" | "side" | "bottom";
type Sizes = Partial<Record<PanelSize, number>>;

const KEY = "biliup.editor.layout";
/** What a dragged size may be, in CSS px (the CSS clamps again against the window). */
export const LIMITS: Readonly<Record<PanelSize, readonly [number, number]>> = { lib: [180, 560], side: [240, 560], bottom: [120, 900] };

const read = (): Sizes => {
  try {
    if (typeof localStorage === "undefined") return {};
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Record<string, unknown>;
    const out: Record<string, number> = {};
    for (const k of ["lib", "side", "bottom"] as const) if (typeof raw[k] === "number" && Number.isFinite(raw[k])) out[k] = clampSize(k, raw[k] as number);
    return out;
  } catch {
    return {};
  }
};

const DEFAULT_SIZES: Sizes = {};
let sizes: Sizes = DEFAULT_SIZES;
const listeners = new Set<() => void>();
const emit = () => listeners.forEach((l) => l());
const write = (key: string, value: unknown) => {
  try { localStorage.setItem(key, JSON.stringify(value)); } catch { /* Browser preferences still work for this session. */ }
};

export const clampSize = (panel: PanelSize, px: number) => Math.round(Math.min(LIMITS[panel][1], Math.max(LIMITS[panel][0], px)));

export const layout = {
  get: () => sizes,
  /** Shown while dragging; saved only when `save` is true (the release). */
  set(panel: PanelSize, px: number | undefined, save: boolean) {
    if (px !== undefined && !Number.isFinite(px)) return;
    const { [panel]: _old, ...rest } = sizes;
    sizes = px === undefined ? rest : { ...rest, [panel]: clampSize(panel, px) };
    if (save) write(KEY, sizes);
    emit();
  },
  reset(panel?: PanelSize) {
    if (panel) layout.set(panel, undefined, true);
    else { sizes = DEFAULT_SIZES; write(KEY, sizes); emit(); }
  },
};

export const useLayout = () =>
  useSyncExternalStore(
    (l) => (listeners.add(l), () => void listeners.delete(l)),
    () => sizes,
    () => DEFAULT_SIZES,
  );

/** The grid's custom properties for the remembered sizes (absent: the CSS default). */
export function layoutStyle(s: Sizes): Record<string, string> {
  const style: Record<string, string> = {};
  if (s.lib !== undefined) style["--lib-col"] = `clamp(180px, ${s.lib}px, 40vw)`;
  if (s.side !== undefined) style["--side-col"] = `clamp(240px, ${s.side}px, 40vw)`;
  if (s.bottom !== undefined) style["--bottom-row"] = `clamp(120px, ${s.bottom}px, 70vh)`;
  return style;
}

// ---- the left rail's tab order (S2, D117) ----

const RAIL_KEY = "biliup.editor.rail";

/** The saved order, with tabs it doesn't name (new ones) at the end and unknown names dropped. */
export function railOrder<T extends string>(saved: unknown, ids: ReadonlyArray<T>): Array<T> {
  const list = Array.isArray(saved) ? saved.filter((x): x is T => typeof x === "string" && ids.includes(x as T)) : [];
  const unique = [...new Set(list)];
  return [...unique, ...ids.filter((id) => !unique.includes(id))];
}

interface RailPreferences { readonly order: ReadonlyArray<string>; readonly hidden: ReadonlyArray<string> }
const DEFAULT_RAIL: RailPreferences = { order: [], hidden: [] };
let railState = DEFAULT_RAIL;
const railListeners = new Set<() => void>();
const railSubscribe = (listener: () => void) => { railListeners.add(listener); return () => { railListeners.delete(listener); }; };
const stringIds = (value: unknown): Array<string> => Array.isArray(value) ? [...new Set(value.filter((id): id is string => typeof id === "string"))] : [];
function readRail(): RailPreferences {
  try {
    const raw = JSON.parse(localStorage.getItem(RAIL_KEY) ?? "{}");
    return { order: stringIds(raw.order), hidden: stringIds(raw.hidden) };
  } catch { return DEFAULT_RAIL; }
}
const saveRail = (next: RailPreferences) => {
  railState = next;
  write(RAIL_KEY, next);
  railListeners.forEach(listener => listener());
};
export const rail = {
  get: () => railState,
  order: (order: ReadonlyArray<string>) => saveRail({ ...railState, order }),
  hide: (hidden: ReadonlyArray<string>) => saveRail({ ...railState, hidden }),
  reset: () => saveRail(DEFAULT_RAIL),
};
export function useRail<T extends string>(ids: ReadonlyArray<T>) {
  const saved = useSyncExternalStore(railSubscribe, rail.get, () => DEFAULT_RAIL);
  return useMemo(() => ({ order: railOrder(saved.order, ids), hidden: saved.hidden.filter((id): id is T => ids.includes(id as T)) }), [saved, ids]);
}

/** Moves `id` to before `before` (undefined: the end); the new order. */
export function moveInRail<T extends string>(order: ReadonlyArray<T>, id: T, before: T | undefined): Array<T> {
  const rest = order.filter((t) => t !== id);
  const at = before === undefined ? rest.length : rest.indexOf(before);
  rest.splice(at < 0 ? rest.length : at, 0, id);
  return rest;
}

// ---- the side panel's tab per project (U14, D180) ----

export type SideTab = "inspector" | "agent" | "history";
const SIDE_TAB_KEY = "biliup.editor.side.tab";
const SIDE_TABS: ReadonlyArray<SideTab> = ["inspector", "agent", "history"];
const MAX_REMEMBERED = 50;

const readSideTabs = (): Record<string, SideTab> => {
  try {
    const raw = JSON.parse(localStorage.getItem(SIDE_TAB_KEY) ?? "{}") as unknown;
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) return {};
    return Object.fromEntries(Object.entries(raw).filter((e): e is [string, SideTab] => SIDE_TABS.includes(e[1] as SideTab)));
  } catch {
    return {};
  }
};

let sideTabs: Record<string, SideTab> = {};
const sideListeners = new Set<() => void>();
const sideSubscribe = (listener: () => void) => { sideListeners.add(listener); return () => { sideListeners.delete(listener); }; };
export const useSideTab = (dir: string) => useSyncExternalStore(sideSubscribe, () => sideTabs[dir] ?? "inspector", (): SideTab => "inspector");

/** Remembers the tab for this project (the newest MAX_REMEMBERED projects are kept). */
export function rememberSideTab(dir: string, tab: SideTab) {
  if (sideTabs[dir] === tab) return;
  const { [dir]: _old, ...rest } = sideTabs;
  const entries = [...Object.entries(rest), [dir, tab] as const].slice(-MAX_REMEMBERED);
  sideTabs = Object.fromEntries(entries);
  write(SIDE_TAB_KEY, sideTabs);
  sideListeners.forEach(listener => listener());
}

// ---- hideable panels and focus mode (#76, D256) ----

/** The parts of the editor that hide: the rail with the left panel, the side panel, the timeline. */
export type Hideable = "lib" | "side" | "bottom";
export interface Panels {
  readonly lib: boolean;
  readonly side: boolean;
  readonly bottom: boolean;
  /** Focus mode: only the preview and the agent beside it (the three flags are kept for after). */
  readonly focus: boolean;
}
const PANELS_KEY = "biliup.editor.layout.hidden";
const NOTHING_HIDDEN: Panels = { lib: false, side: false, bottom: false, focus: false };

const readPanels = (): Panels => {
  try {
    if (typeof localStorage === "undefined") return NOTHING_HIDDEN;
    const raw = JSON.parse(localStorage.getItem(PANELS_KEY) ?? "{}") as Record<string, unknown>;
    return { lib: raw.lib === true, side: raw.side === true, bottom: raw.bottom === true, focus: raw.focus === true };
  } catch {
    return NOTHING_HIDDEN;
  }
};

let hidden: Panels = NOTHING_HIDDEN;
const panelListeners = new Set<() => void>();
const savePanels = (next: Panels) => {
  hidden = next;
  write(PANELS_KEY, next);
  panelListeners.forEach((l) => l());
};

/** Whether a part shows now (focus mode hides all three). */
export const panelShown = (p: Panels, panel: Hideable) => !p.focus && !p[panel];

export const panels = {
  get: () => hidden,
  /** Hides a shown part, shows a hidden one; from focus mode it leaves focus with that part shown. */
  toggle(panel: Hideable) {
    if (hidden.focus) return savePanels({ ...hidden, focus: false, [panel]: false });
    savePanels({ ...hidden, [panel]: !hidden[panel] });
  },
  show(panel: Hideable) {
    if (panelShown(hidden, panel)) return;
    savePanels({ ...hidden, focus: false, [panel]: false });
  },
  focus(on = !hidden.focus) {
    if (on !== hidden.focus) savePanels({ ...hidden, focus: on });
  },
  /** Everything back (the top bar's way out). */
  showAll: () => savePanels(NOTHING_HIDDEN),
};

export const subscribePanels = (l: () => void) => (panelListeners.add(l), () => void panelListeners.delete(l));
export const usePanels = () => useSyncExternalStore(subscribePanels, () => hidden, () => NOTHING_HIDDEN);

let restored = false;
/** Browser preferences are restored when the native editor mounts, never during server rendering. */
export function restoreWorkspaceLayout() {
  if (restored || typeof window === "undefined") return;
  restored = true;
  sizes = read(); railState = readRail(); sideTabs = readSideTabs(); hidden = readPanels();
  emit();
  railListeners.forEach(listener => listener());
  sideListeners.forEach(listener => listener());
  panelListeners.forEach(listener => listener());
}

/**
 * The grid's tracks with parts hidden (none hidden: the CSS default). The rail and the left panel
 * go together. Focus mode (D270): the preview on the left, the agent in the side column at full
 * height (--focus-col), the timeline's row at 0. Every track keeps the shape of its shown form (a
 * length, or minmax with a length) so the grid's transition interpolates it (D271: the timeline's
 * row was `minmax(..)` shown and `0px` hidden, which doesn't interpolate, so it jumped).
 */
export function panelsStyle(p: Panels): Record<string, string> {
  if (!p.focus && !p.lib && !p.side && !p.bottom) return {};
  const lib = panelShown(p, "lib");
  const side = p.focus ? "var(--focus-col)" : panelShown(p, "side") ? "var(--side-col)" : "0px";
  const bottom = panelShown(p, "bottom") ? "minmax(0, var(--bottom-row))" : "minmax(0, 0px)";
  return {
    gridTemplateColumns: `${lib ? "56px" : "0px"} ${lib ? "var(--lib-col)" : "0px"} minmax(0, 1fr) ${side}`,
    gridTemplateRows: `var(--editor-top-row, 40px) minmax(0, 1fr) ${bottom}`,
  };
}
