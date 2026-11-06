import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { isTextComposition } from '@/app/lib/editor/keyboard';
import { focusEditor } from '@/app/lib/editor/environment';
import { currentEditorStore } from '@/app/lib/editor/state/store/core';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The timeline's right-click menu (T7, D126): an in-app menu that looks and behaves like a native
// one (opens at the pointer, kept inside the window, ↑/↓/Home/End move, → opens a submenu, ← and Esc
// close, Enter/Space choose, typing a letter jumps to the next entry with it), so the app tests and
// the remote browser get the same menu. One menu open at a time, from anywhere (openMenu).
import { Check, ChevronRight } from "lucide-react";
import React, { useEffect, useId, useLayoutEffect, useRef, useState, useSyncExternalStore } from "react";

export interface MenuEntry {
  readonly id: string;
  readonly label: string;
  readonly shortcut?: string;
  readonly disabled?: boolean;
  readonly checked?: boolean;
  readonly danger?: boolean;
  readonly run?: () => void;
  readonly submenu?: ReadonlyArray<MenuEntry | "-">;
}
export type MenuItems = ReadonlyArray<MenuEntry | "-">;

let open: { id:number; x: number; y: number; items: MenuItems; label: string } | undefined;
let serial=0;
let releaseOwner: (() => void) | undefined;
const listeners = new Set<() => void>();
const notify = () => {
  for (const l of listeners) l();
};
/** Open a menu at a point (client px). */
export function openMenu(x: number, y: number, items: MenuItems, label = "Menu") {
  releaseOwner?.();
  releaseOwner = undefined;
  const owner = currentEditorStore();
  const initial = owner?.getSnapshot();
  const valid = () => {
    const state = owner?.getSnapshot();
    return currentEditorStore() === owner && state?.snapshot?.dir === initial?.snapshot?.dir
      && state?.snapshot?.revision === initial?.snapshot?.revision
      && state?.connection === initial?.connection && state?.biliupCanEdit === initial?.biliupCanEdit;
  };
  const owned = (rows: MenuItems): MenuItems => rows.map(entry => entry === '-' ? entry : {
    ...entry,
    ...(entry.run ? { run: () => { if (valid()) entry.run?.(); } } : {}),
    ...(entry.submenu ? { submenu: owned(entry.submenu) } : {}),
  });
  open = { id:++serial, x, y, items: owned(items), label };
  releaseOwner = owner?.subscribe(() => { if (!valid()) closeMenu(); });
  notify();
}
export const closeMenu = () => {
  releaseOwner?.();
  releaseOwner = undefined;
  if (!open) return;
  const focused = document.activeElement?.closest('[role=menu]');
  open = undefined;
  notify();
  if (focused) focusEditor();
};
const useOpenMenu = () => useSyncExternalStore((l) => (listeners.add(l), () => void listeners.delete(l)), () => open);

/** Mounted once (in the editor): draws the open menu, if any. */
export function ContextMenuHost() {
  useEditorLanguage();
  const menu = useOpenMenu();
  useEffect(() => () => closeMenu(), []);
  useEffect(() => {
    if (!menu) return;
    const away = (e: PointerEvent) => {
      if (!(e.target as HTMLElement | null)?.closest?.(".ctx-menu")) closeMenu();
    };
    const resize = () => closeMenu();
    window.addEventListener("pointerdown", away, true);
    window.addEventListener("resize", resize);
    window.addEventListener("blur", resize);
    return () => {
      window.removeEventListener("pointerdown", away, true);
      window.removeEventListener("resize", resize);
      window.removeEventListener("blur", resize);
    };
  }, [menu]);
  if (!menu) return null;
  return <MenuList key={menu.id} items={menu.items} x={menu.x} y={menu.y} label={editorText(menu.label)} level={0} onClose={closeMenu} />;
}

const enabled = (items: MenuItems) => items.map((e, i) => (e !== "-" && !e.disabled ? i : -1)).filter((i) => i >= 0);

function MenuList({ items, x, y, anchorLeft, label, level, onClose, onBack }: { items: MenuItems; x: number; y: number; anchorLeft?:number; label: string; level: number; onClose: () => void; onBack?: () => void }) {
  useEditorLanguage();
  const menuId=useId();
  const ref = useRef<HTMLDivElement>(null);
  const rowAt=(index:number)=>ref.current?.querySelector<HTMLElement>(`[data-menu-index="${index}"]`);
  const [pos, setPos] = useState({ x, y });
  const [active, setActiveState] = useState(level > 0 ? (enabled(items)[0] ?? -1) : -1);
  // Keys can come faster than renders (type-ahead): they read the latest entry from here.
  const activeRef = useRef(active);
  const pointer = useRef<{ x: number; y: number } | undefined>(undefined);
  const setActive = (i: number) => {
    activeRef.current = i;
    setActiveState(i);
  };
  const [sub, setSub] = useState<{ index: number; x: number; y: number; left:number } | undefined>();
  const closeSub = () => {
    setSub(undefined);
    if (sub) ref.current?.focus({ preventScroll: true });
  };
  // Kept inside the window: flipped left / up when it would run off.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const flipped=x+r.width>window.innerWidth-4?(anchorLeft===undefined?x-r.width:anchorLeft-r.width-2):x;
    const nx=Math.max(4,Math.min(flipped,window.innerWidth-r.width-4));
    const ny=Math.max(4,Math.min(y,window.innerHeight-r.height-4));
    setPos({ x: nx, y: ny });
    // Not scrolled to: it's drawn where it fits only after this render (U13).
    el.focus({ preventScroll: true });
  }, [x, y, anchorLeft]);
  useLayoutEffect(()=>{if(active>=0)rowAt(active)?.scrollIntoView({block:'nearest',inline:'nearest'})},[active]);
  const choose = (index: number) => {
    const entry = items[index];
    if (!entry || entry === "-" || entry.disabled) return;
    if (entry.submenu) {
      const row = rowAt(index);
      const r = row?.getBoundingClientRect();
      setSub({ index, x: (r?.right ?? pos.x) + 2, y: (r?.top ?? pos.y) - 4, left:r?.left??pos.x });
      return;
    }
    onClose();
    entry.run?.();
  };
  const onKey = (e: React.KeyboardEvent) => {
    e.stopPropagation();
    if (sub || isTextComposition(e.nativeEvent)) return;
    if(e.key==='Tab'){onClose();return;}
    const list = enabled(items);
    const active = activeRef.current;
    const at = list.indexOf(active);
    const stop = () => (e.preventDefault(), e.stopPropagation());
    if (e.key === "ArrowDown") (stop(), setActive(list[(at + 1) % list.length] ?? -1));
    else if (e.key === "ArrowUp") (stop(), setActive(list[(at - 1 + list.length) % list.length] ?? -1));
    else if (e.key === "Home") (stop(), setActive(list[0] ?? -1));
    else if (e.key === "End") (stop(), setActive(list.at(-1) ?? -1));
    else if (e.key === "Enter" || e.key === " ") (stop(), active >= 0 && choose(active));
    else if (e.key === "ArrowRight") {
      stop();
      const entry = items[active];
      if (entry && entry !== "-" && entry.submenu) choose(active);
    } else if (e.key === "ArrowLeft" && onBack) (stop(), onBack());
    else if (e.key === "Escape") (stop(), onBack ? onBack() : onClose());
    else if (e.key.length === 1 && /\S/.test(e.key)) {
      // Type-ahead: the next entry starting with that letter.
      const k = e.key.toLowerCase();
      const order = [...list.slice(at + 1), ...list.slice(0, at + 1)];
      const hit = order.find((i) => editorText((items[i] as MenuEntry).label).toLocaleLowerCase().startsWith(k));
      if (hit !== undefined) (stop(), setActive(hit));
    }
  };
  const subEntry = sub ? (items[sub.index] as MenuEntry) : undefined;
  return (
    <>
      <div
        className="ctx-menu"
        role="menu"
        aria-label={label}
        aria-activedescendant={active>=0?`${menuId}-${active}`:undefined}
        data-testid={level === 0 ? "context-menu" : "context-submenu"}
        tabIndex={-1}
        ref={ref}
        style={{ left: pos.x, top: pos.y }}
        onKeyDown={onKey}
        onScroll={closeSub}
        onContextMenu={(e) => e.preventDefault()}
      >
        {items.map((entry, i) =>
          entry === "-" ? (
            <div key={`sep-${i}`} className="ctx-sep" role="separator" />
          ) : (
            <div
              key={entry.id}
              id={`${menuId}-${i}`}
              data-menu-index={i}
              role={entry.checked !== undefined ? "menuitemcheckbox" : "menuitem"}
              aria-checked={entry.checked}
              aria-disabled={entry.disabled || undefined}
              aria-haspopup={entry.submenu ? "menu" : undefined}
              aria-expanded={entry.submenu?sub?.index===i:undefined}
              data-testid={`menu-${entry.id}`}
              className={`ctx-item ${i === active ? "active" : ""} ${entry.disabled ? "disabled" : ""} ${entry.danger ? "danger" : ""}`}
              onPointerMove={(e) => {
                // Only a pointer that really moves takes over: a menu opened from the keyboard can
                // appear under a resting pointer (the browser then sends moves without movement).
                const last = pointer.current;
                pointer.current = { x: e.clientX, y: e.clientY };
                if (!last || (last.x === e.clientX && last.y === e.clientY) || activeRef.current === i) return;
                setActive(entry.disabled ? -1 : i);
                if (entry.submenu && !entry.disabled) choose(i);
                else closeSub();
              }}
              onClick={() => choose(i)}
            >
              <span className="ctx-check">{entry.checked && <Check />}</span>
              <span className="ctx-label">{editorText(entry.label)}</span>
              {entry.shortcut && <span className="ctx-key">{entry.shortcut}</span>}
              {entry.submenu && <ChevronRight className="ctx-more" />}
            </div>
          ),
        )}
      </div>
      {sub && subEntry?.submenu && (
        <MenuList
          key={subEntry.id}
          items={subEntry.submenu}
          x={sub.x}
          y={sub.y}
          anchorLeft={sub.left}
          label={editorText(subEntry.label)}
          level={level + 1}
          onClose={onClose}
          onBack={closeSub}
        />
      )}
    </>
  );
}
