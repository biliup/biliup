import { editorEvents } from "@/app/lib/editor/environment";
import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { editorRoot } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The app's one tooltip (#86, D239): an action's name and its keyboard shortcut, shown after a
// short rest of the pointer (or on keyboard focus), above the control and kept inside the window.
// Wrap a single control: <Tooltip label="Split at the playhead" shortcut="split">. A `shortcut`
// shows the keymap's current keys for that action (at most two), updated when they're rebound (#78).
// It names the control (aria-label) and replaces the native title, so a control shows one tip.
// Keys come from the keymap (editor/keys.ts SHORTCUTS, timeline/tools.ts TOOLS), never typed here.
import { cloneElement, useEffect, useId, useLayoutEffect, useRef, useState, type ReactElement, type SyntheticEvent } from "react";
import { createPortal } from "react-dom";

import { useBindings, keysOf, displayKey, type ShortcutId } from "@/app/ui/editor/workspace/keymap";

/** How long the pointer rests on a control before its tip shows. */
export const TIP_DELAY_MS = 450;
/** A tip shown less than this long ago makes the next one show at once (moving along a toolbar). */
const WARM_MS = 600;
let lastHidden = 0;

type Handler = (e: SyntheticEvent<HTMLElement>) => void;
type ChildProps = {
  onPointerEnter?: Handler;
  onPointerLeave?: Handler;
  onPointerDown?: Handler;
  onFocus?: Handler;
  onBlur?: Handler;
  title?: string;
  "aria-label"?: string;
  "aria-describedby"?: string;
};

/** Keys a tooltip shows for an action: its first two. */
export const TIP_KEYS = 2;

export function Tooltip({ label, keys: literal = [], shortcut, children }: { label: string; keys?: ReadonlyArray<string>; shortcut?: ShortcutId; children: ReactElement<ChildProps> }) {
  useEditorLanguage();
  label = editorText(label);
  useBindings();
  const keys = shortcut ? keysOf(shortcut).slice(0, TIP_KEYS) : literal;
  const id = useId();
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const tip = useRef<HTMLDivElement>(null);
  const [at, setAt] = useState<{ left: number; top: number } | undefined>(undefined);

  const hide = () => {
    clearTimeout(timer.current);
    setAnchor((a) => {
      if (a) lastHidden = performance.now();
      return null;
    });
    setAt(undefined);
  };
  const open = (el: HTMLElement) => {
    clearTimeout(timer.current);
    if (performance.now() - lastHidden < WARM_MS) setAnchor(el);
    else timer.current = setTimeout(() => setAnchor(el), TIP_DELAY_MS);
  };

  useEffect(() => () => clearTimeout(timer.current), []);
  // Placed once its size is known: centred above the control, below it if there's no room, inside the window.
  useLayoutEffect(() => {
    if (!anchor || !tip.current) return;
    const r = anchor.getBoundingClientRect();
    const w = tip.current.offsetWidth;
    const h = tip.current.offsetHeight;
    const left = Math.max(6, Math.min(window.innerWidth - w - 6, r.left + r.width / 2 - w / 2));
    const above = r.top - 6 - h;
    setAt({ left, top: above >= 6 ? above : Math.min(window.innerHeight - h - 6, r.bottom + 6) });
  }, [anchor, label, keys.join("\n")]);
  useEffect(() => {
    if (!anchor) return;
    const key = (e: KeyboardEvent) => e.key === "Escape" && hide();
    editorEvents.addEventListener("keydown", key, true);
    window.addEventListener("scroll", hide, true);
    window.addEventListener("blur", hide);
    return () => {
      editorEvents.removeEventListener("keydown", key, true);
      window.removeEventListener("scroll", hide, true);
      window.removeEventListener("blur", hide);
    };
  }, [anchor]);

  const p = children.props;
  const chain =
    (own: Handler | undefined, mine: Handler): Handler =>
    (e) => {
      own?.(e);
      mine(e);
    };
  const control = cloneElement(children, {
    title: undefined,
    "aria-label": p["aria-label"] ?? label,
    "aria-describedby": anchor ? id : p["aria-describedby"],
    onPointerEnter: chain(p.onPointerEnter, (e) => open(e.currentTarget)),
    onPointerLeave: chain(p.onPointerLeave, hide),
    onPointerDown: chain(p.onPointerDown, hide),
    onFocus: chain(p.onFocus, (e) => e.currentTarget.matches(":focus-visible") && open(e.currentTarget)),
    onBlur: chain(p.onBlur, hide),
  });
  return (
    <>
      {control}
      {anchor &&
        createPortal(
          <div ref={tip} id={id} role="tooltip" className="tooltip" data-testid="tooltip" style={{ left: at?.left ?? -9999, top: at?.top ?? -9999 }}>
            <span className="tooltip-label">{label}</span>
            {keys.length > 0 && (
              <span className="tooltip-keys" data-testid="tooltip-keys">
                {keys.map((k) => (
                  <kbd key={k}>{displayKey(k)}</kbd>
                ))}
              </span>
            )}
          </div>,
          editorRoot(),
        )}
    </>
  );
}
