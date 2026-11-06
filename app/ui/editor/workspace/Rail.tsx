import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The left rail: the library's tabs. D209 (issue #26): a plain drag (past a few pixels) moves a tab,
// a line shows where it lands; a click still switches. Right-click: Move up, Move down, Hide, and
// Reset order (which also shows the hidden ones). Hidden tabs are in the More menu at the end. The
// order and the hidden ones are remembered in this browser (S2, D117).
import { AudioWaveform, Blend, BookOpen, CircleDot, Ellipsis, Film,  Music, Palette, ScrollText, Shapes, Sparkles, SwatchBook, Type, type LucideIcon } from "lucide-react";
import type React from "react";
import { useEffect, useRef, useState } from "react";

import type { LibraryTab } from "@/app/ui/editor/panels/Library";
import { openMenu, type MenuItems } from "@/app/ui/editor/timeline/ContextMenu";
import { useRail, rail, moveInRail } from "@/app/ui/editor/workspace/workspaceLayout";
import { currentEditorStore } from "@/app/lib/editor/state/store/core";
import { beginPointerGesture } from "./pointerGesture";

/** Pixels a press moves before it's a drag, not a click. */
export const DRAG_THRESHOLD = 5;

const TABS: ReadonlyArray<{ readonly id: LibraryTab; readonly label: string; readonly title: string; readonly icon: LucideIcon }> = [
  { id: "media", get label() { return editorText("Media"); }, get title() { return editorText("Media"); }, icon: Film },
  // U11 (D177): the transcript of what's said on the main track, next to the media it comes from.
  { id: "transcript", get label() { return editorText("Transcript"); }, get title() { return editorText("Transcript"); }, icon: ScrollText },
  { id: "text", get label() { return editorText("Text"); }, get title() { return editorText("Text"); }, icon: Type },
  { id: "elements", get label() { return editorText("Elements"); }, get title() { return editorText("Elements"); }, icon: Shapes },
  { id: "audio", get label() { return editorText("Audio"); }, get title() { return editorText("Audio"); }, icon: Music },
  { id: "sfx", get label() { return editorText("Sounds"); }, get title() { return editorText("Sound effects"); }, icon: AudioWaveform },
  { id: "transitions", get label() { return editorText("Transitions"); }, get title() { return editorText("Transitions"); }, icon: Blend },
  { id: "effects", get label() { return editorText("Effects"); }, get title() { return editorText("Filters and effects"); }, icon: Palette },
  { id: "capture", get label() { return editorText("Capture"); }, get title() { return editorText("Capture"); }, icon: CircleDot },
  { id: "generate", get label() { return editorText("Generate"); }, get title() { return editorText("Generate"); }, icon: Sparkles },
  // D186: brand kits and skills, and the ones this project uses (a saved order gets it at the end).
  { id: "brand", get label() { return editorText("Brand"); }, get title() { return editorText("Brand kits"); }, icon: SwatchBook },
  // Shared editing skills are available as slash commands in the assistant.
  { id: "skills", get label() { return editorText("Skills"); }, get title() { return editorText("Skills"); }, icon: BookOpen },
];
const IDS = TABS.map((t) => t.id);
const info = (id: LibraryTab) => TABS.find((t) => t.id === id)!;

export function Rail({ tab, onTab, visible = true }: { tab: LibraryTab; onTab: (tab: LibraryTab) => void; visible?: boolean }) {
  useEditorLanguage();
  const { order, hidden } = useRail(IDS);
  // The tab being dragged and the one it would land before ("" = the end).
  const [drag, setDrag] = useState<{ id: LibraryTab; before: LibraryTab | "" } | undefined>();
  // A drag just ended: the click that follows the release doesn't switch tabs.
  const swallowClick = useRef(false);
  const nav = useRef<HTMLElement>(null);
  const stop = useRef<(() => void) | undefined>(undefined);
  const mounted = useRef(true);
  const enabled = useRef(visible);
  enabled.current = visible;
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; stop.current?.(); };
  }, []);
  useEffect(() => { stop.current?.(); }, [visible, order, hidden]);
  const saveOrder = rail.order;
  const saveHidden = rail.hide;
  const reset = rail.reset;

  const shown = order.filter((id) => !hidden.includes(id));

  /** Where a pointer at clientY would drop: before the first shown tab whose middle is below it. */
  const dropBefore = (y: number, id: LibraryTab): LibraryTab | "" => {
    for (const el of nav.current?.querySelectorAll<HTMLElement>("[data-rail]") ?? []) {
      const r = el.getBoundingClientRect();
      if (y < r.top + r.height / 2) return el.dataset.rail as LibraryTab;
    }
    return id === shown.at(-1) ? id : "";
  };

  const onPointerDown = (e: React.PointerEvent<HTMLButtonElement>, id: LibraryTab) => {
    const owner = currentEditorStore();
    if (e.button !== 0 || !visible || !owner) return;
    stop.current?.();
    swallowClick.current = false;
    const preferences = rail.get();
    const button = e.currentTarget;
    const x0 = e.clientX;
    const y0 = e.clientY;
    let active = false;
    let before: LibraryTab | "" = id;
    const move = (ev: PointerEvent) => {
      if (!active) {
        if (Math.hypot(ev.clientX - x0, ev.clientY - y0) < DRAG_THRESHOLD) return;
        active = true;
        try {
          button.setPointerCapture(e.pointerId);
        } catch {}
      }
      before = dropBefore(ev.clientY, id);
      setDrag({ id, before });
    };
    const up = (event: PointerEvent) => {
      const moved = active;
      if (active) before = dropBefore(event.clientY, id);
      finish();
      if (!moved) return;
      // Dropped right where it was (before itself or the next one): nothing moves.
      const target = before === "" ? undefined : before;
      if (target === id) return;
      const next = moveInRail(order, id, target);
      if (next.join() !== order.join()) saveOrder(next);
    };
    const finish = () => {
      stop.current = undefined;
      if (active) swallowClick.current = true;
      if (mounted.current) setDrag(undefined);
      button.removeEventListener("lostpointercapture", cancel);
      if (button.hasPointerCapture(e.pointerId)) button.releasePointerCapture(e.pointerId);
    };
    const cancel = () => stop.current?.();
    stop.current = beginPointerGesture(e, { move, commit: up, cancel: finish },
      () => mounted.current && enabled.current && currentEditorStore() === owner && rail.get() === preferences, owner.subscribe);
    button.addEventListener("lostpointercapture", cancel);
  };

  const menuFor = (id: LibraryTab): MenuItems => {
    const i = shown.indexOf(id);
    const step = (by: -1 | 1) => {
      const neighbour = shown[i + by];
      if (!neighbour) return;
      // Up: before the one above; down: before the one after the one below.
      saveOrder(moveInRail(order, id, by < 0 ? neighbour : shown[i + 2]));
    };
    return [
      { id: "rail-up", get label() { return editorText("Move up"); }, disabled: i <= 0, run: () => step(-1) },
      { id: "rail-down", get label() { return editorText("Move down"); }, disabled: i < 0 || i >= shown.length - 1, run: () => step(1) },
      "-",
      { id: "rail-hide", label: editorText("Hide {0}", { 0: info(id).label }), disabled: shown.length <= 1, run: () => saveHidden([...hidden, id]) },
      { id: "rail-reset", get label() { return editorText("Reset order"); }, run: reset },
    ];
  };

  const moreMenu = (): MenuItems => [
    ...hidden.map((id) => ({ id: `rail-open-${id}`, label: info(id).title, checked: tab === id, run: () => onTab(id) })),
    "-",
    ...hidden.map((id) => ({ id: `rail-show-${id}`, label: editorText("Show {0} in the rail", { 0: info(id).label }), run: () => saveHidden(hidden.filter((h) => h !== id)) })),
    { id: "rail-reset", get label() { return editorText("Reset order"); }, run: reset },
  ];

  return (
    // No overlay scrollbar on the rail (D263): it drew a grey bar along the left panel's edge whenever the pointer was over a rail
    // a little too tall for the window; the wheel still scrolls it.
    <nav className={`rail ${drag ? "dragging" : ""}`} ref={nav} data-testid="rail" data-scrollbars="off" data-order={order.join(",")} data-hidden={hidden.join(",")}>
      {shown.map((id) => {
        const t = info(id);
        const Icon = t.icon;
        return (
          <button
            key={id}
            className={`${tab === id ? "active" : ""} ${drag?.id === id ? "lifted" : ""} ${drag && drag.before === id && drag.id !== id ? "drop-before" : ""}`}
            data-testid={`rail-${id}`}
            data-rail={id}
            title={editorText(t.title)}
            onPointerDown={(e) => onPointerDown(e, id)}
            onContextMenu={(e) => {
              e.preventDefault();
              openMenu(e.clientX, e.clientY, menuFor(id), editorText("{0} tab", { 0: t.label }));
            }}
            onClick={(event) => {
              if (swallowClick.current && event.detail !== 0) return void (swallowClick.current = false);
              swallowClick.current = false;
              onTab(id);
            }}
          >
            <Icon />
            <span>{editorText(t.label)}</span>
          </button>
        );
      })}
      {drag && drag.before === "" && <div className="rail-drop-end" data-testid="rail-drop-indicator" />}
      {hidden.length > 0 && (
        <button
          className={`rail-more ${hidden.includes(tab) ? "active" : ""}`}
          data-testid="rail-more"
          title={editorText("More: {0}", { 0: hidden.map((id) => info(id).label).join(", ") })}
          onClick={(e) => {
            const r = e.currentTarget.getBoundingClientRect();
            openMenu(r.right + 4, r.top, moreMenu(), editorText("More tabs"));
          }}
        >
          <Ellipsis />
          <span>{editorText("More")}</span>
        </button>
      )}
    </nav>
  );
}
