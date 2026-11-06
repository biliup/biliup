import { editorContains, editorDialogOpen, editorEvents, focusEditor } from "@/app/lib/editor/environment";
import { isTextComposition } from '@/app/lib/editor/keyboard';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Issue #48: Space plays the timeline unless a text field is really being typed in. Focus ends up in
// fields the user isn't typing in: the timeline and the stage cancel their presses (so the field
// last typed in keeps focus), and some fields are focused for the user (Add to chat, a double click
// on a canvas element, autofocus). These rules keep Space (and the other shortcuts) for the editor:
// - a press outside the focused text field takes focus off it, unless the press is meant to keep it
//   (its mousedown is cancelled on purpose: the Accent button, a mention option, the leave dialog);
// - a field counts as "being typed in" only once the user pressed on it, tabbed to it or typed a key
//   in it; a number field never takes a Space;
// - a button pressed with the mouse doesn't keep focus (Enter/Space don't press it again);
// - Esc in a text field that didn't handle it leaves the field.
import { isTextEntry } from "@/app/lib/editor/state/store";

/** The field the user chose to type in (pressed on, tabbed to or typed in). */
let chosen: Element | null = null;

const fieldOf = (target: EventTarget | null): HTMLElement | undefined => {
  if (!(target instanceof Element)) return undefined;
  if (isTextEntry(target)) return target as HTMLElement;
  const editable = target instanceof HTMLElement && target.isContentEditable ? target.closest<HTMLElement>("[contenteditable]") : undefined;
  if (editable) return editable;
  // A press on a field's label (a NumberField's name, the row around a field) is a press on the field.
  const control = target.closest("label")?.control;
  return control && isTextEntry(control) ? (control as HTMLElement) : undefined;
};

const numeric = (el: Element) => el instanceof HTMLInputElement && (el.type === "number" || el.inputMode === "decimal" || el.inputMode === "numeric");

/** True while Space belongs to the focused field: a text field the user chose to type in. */
export function isTyping(el: EventTarget | null): boolean {
  if (!(el instanceof Element) || !isTextEntry(el) || numeric(el)) return false;
  return chosen !== null && (chosen === el || chosen.contains(el));
}

/** Installs the focus rules; returns the cleanup. */
export function installFocusRules(): () => void {
  let activeRules = true;
  chosen = null;
  let pressId = 0;
  let keptBy = -1;
  // The press whose own mousedown already took focus off the field (to a button, say). Something it
  // does may put focus back in the field on purpose (Make from reference, Add to chat): that stays.
  let movedBy = -1;
  let pressed: Element | null = null;
  let tabbing = false;
  let byPointer = false;
  let before: Element | null = null;
  const onPointerDown = (e: PointerEvent) => {
    const id = ++pressId;
    if (!editorContains(e.target) || editorDialogOpen()) { chosen = null; pressed = null; return; }
    byPointer = true;
    tabbing = false;
    const field = fieldOf(e.target);
    if (field) return void (chosen = field);
    before = chosen;
    chosen = null;
    const active = document.activeElement;
    pressed = active;
    if (!(active instanceof HTMLElement) || !isTextEntry(active) || active.contains(e.target as Node | null)) return;
    // After the press is handled: a cancelled mousedown means "keep typing there"; a cancelled
    // pointerdown (no mousedown at all) or an ordinary press elsewhere means the field is done with.
    // The timer can fire after the click (a busy or throttled renderer, #89): by then a focus the
    // click gave back to the field is the app's, not the field the press left.
    setTimeout(() => {
      if (!activeRules || pressId !== id || editorDialogOpen() || keptBy === id || movedBy === id || document.activeElement !== active) return;
      focusEditor();
    }, 0);
  };
  const onMouseDown = (e: MouseEvent) => {
    if (!editorContains(e.target) || editorDialogOpen()) return;
    if (!e.defaultPrevented || fieldOf(e.target)) return;
    // Kept on purpose: still the field the user is typing in.
    keptBy = pressId;
    chosen = before;
  };
  const onKeyDownCapture = (e: KeyboardEvent) => {
    if (editorDialogOpen()) return;
    byPointer = false;
    tabbing = e.key === "Tab";
    if (isTextComposition(e)) { const field = fieldOf(e.target); if (field) chosen = field; return; }
    if (e.key === " " || e.key === "Escape" || e.key === "Tab" || e.metaKey || e.ctrlKey || ["Shift", "Alt", "Meta", "Control", "CapsLock"].includes(e.key)) return;
    const field = fieldOf(e.target);
    if (field) chosen = field;
  };
  const onMouseUp = () => {
    if (pressed && document.activeElement !== pressed) movedBy = pressId;
    pressed = null;
  };
  const onFocusIn = (e: FocusEvent) => {
    if (tabbing && fieldOf(e.target)) chosen = e.target as Element;
  };
  const onKeyDown = (e: KeyboardEvent) => {
    // Esc in a field that didn't use it: off the field, like a click elsewhere (after the field's
    // own Esc handling, which may put the old text back first).
    if (e.key !== "Escape" || e.defaultPrevented || isTextComposition(e) || editorDialogOpen() || !isTextEntry(e.target)) return;
    const el = e.target as HTMLElement;
    requestAnimationFrame(() => activeRules && !e.defaultPrevented && document.activeElement === el && focusEditor());
  };
  const onClick = (e: MouseEvent) => {
    if (!editorContains(e.target) || editorDialogOpen()) return;
    // A mouse press on a button (detail > 0; a key press has detail 0) leaves nothing focused.
    const el = document.activeElement;
    if (e.detail > 0 && el instanceof HTMLButtonElement && el.contains(e.target as Node | null)) focusEditor();
  };
  const onChange = (e: Event) => {
    if (!editorContains(e.target) || editorDialogOpen()) return;
    // A list or a checkbox picked with the mouse doesn't keep the keys either.
    const el = e.target;
    if (byPointer && (el instanceof HTMLSelectElement || (el instanceof HTMLInputElement && (el.type === "checkbox" || el.type === "radio")))) focusEditor();
  };
  window.addEventListener("pointerdown", onPointerDown, true);
  window.addEventListener("mousedown", onMouseDown);
  window.addEventListener("mouseup", onMouseUp, true);
  editorEvents.addEventListener("keydown", onKeyDownCapture, true);
  editorEvents.addEventListener("keydown", onKeyDown);
  editorEvents.addEventListener("focusin", onFocusIn);
  window.addEventListener("click", onClick);
  window.addEventListener("change", onChange);
  return () => {
    activeRules = false;
    chosen = null;
    window.removeEventListener("pointerdown", onPointerDown, true);
    window.removeEventListener("mousedown", onMouseDown);
    window.removeEventListener("mouseup", onMouseUp, true);
    editorEvents.removeEventListener("keydown", onKeyDownCapture, true);
    editorEvents.removeEventListener("keydown", onKeyDown);
    editorEvents.removeEventListener("focusin", onFocusIn);
    window.removeEventListener("click", onClick);
    window.removeEventListener("change", onChange);
  };
}
