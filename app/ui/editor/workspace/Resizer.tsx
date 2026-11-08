import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { editorRoot } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { useEffect, useRef, type PointerEvent as ReactPointerEvent } from "react";
import { currentEditorStore } from "@/app/lib/editor/state/store/core";
import { beginPointerGesture } from "./pointerGesture";

import { layout, panels, panelShown, type PanelSize } from "@/app/ui/editor/workspace/workspaceLayout";

/**
 * A drag handle on a panel's edge (G5): the left panel's right edge, the inspector's left edge,
 * the timeline's top edge; a grip line shows under the pointer and while dragging (F31). Dragging resizes live; the size is saved on release; a double click
 * puts the default back.
 */
export function Resizer({ panel, visible = true }: { panel: PanelSize; visible?: boolean }) {
  useEditorLanguage();
  const stop = useRef<(() => void) | undefined>(undefined);
  const enabled = useRef(visible);
  enabled.current = visible;
  useEffect(() => { if (!visible) stop.current?.(); }, [visible]);
  useEffect(() => () => stop.current?.(), []);
  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    const owner = currentEditorStore();
    const dir = owner?.getSnapshot().snapshot?.dir;
    if (e.button !== 0 || !visible || !owner || !dir) return;
    stop.current?.();
    e.preventDefault();
    const handle = e.currentTarget;
    const app = handle.closest<HTMLElement>(".app");
    const rect = app?.querySelector(panel === "lib" ? ".library" : panel === "side" ? ".side" : ".bottom")?.getBoundingClientRect();
    if (!rect) return;
    const root = editorRoot();
    const previous = layout.get()[panel];
    const initial = panel === "bottom" ? rect.height : rect.width;
    const start = panel === "bottom" ? e.clientY : e.clientX;
    const sizeAt = (ev: PointerEvent) => initial + (panel === "bottom" ? start - ev.clientY : panel === "side" ? start - ev.clientX : ev.clientX - start);
    let previewed = false;
    const finish = () => {
      stop.current = undefined;
      handle.removeEventListener("lostpointercapture", cancel);
      if (handle.hasPointerCapture(e.pointerId)) handle.releasePointerCapture(e.pointerId);
      root.classList.remove("resizing");
      handle.classList.remove("dragging");
    };
    const cancel = () => stop.current?.();
    stop.current = beginPointerGesture(e, {
      move: event => {
        if (!previewed && sizeAt(event) === initial) return;
        previewed = true;
        layout.set(panel, sizeAt(event), false);
      },
      commit: event => { finish(); if (previewed || sizeAt(event) !== initial) layout.set(panel, sizeAt(event), true); },
      cancel: () => { finish(); if (previewed) layout.set(panel, previous, false); },
    }, () => enabled.current && currentEditorStore() === owner && owner.getSnapshot().snapshot?.dir === dir && panelShown(panels.get(), panel), owner.subscribe);
    handle.addEventListener("lostpointercapture", cancel);
    root.classList.add("resizing");
    handle.classList.add("dragging");
    try { handle.setPointerCapture(e.pointerId); } catch { cancel(); }
  };
  return <div className={`resizer ${panel === "bottom" ? "h" : "v"} resizer-${panel}`} data-testid={`resize-${panel}`} title={editorText("Drag to resize, double-click to reset")} onPointerDown={onPointerDown} onDoubleClick={() => { stop.current?.(); layout.reset(panel); }} />;
}
