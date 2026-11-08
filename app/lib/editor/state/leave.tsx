import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { editorRoot } from "@/app/lib/editor/environment";
import { editorEvents } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Project navigation protects browser drafts, uploads and recordings. Background jobs belong
// to the biliup service and are neither waited on nor cancelled when this page closes.
import { useEffect, useState, useSyncExternalStore } from "react";

import { cancelTake, getRecorder, stopTake } from "@/app/ui/editor/capture/recorder";
import { leaveWords, pendingWork, type LeaveReason, type PendingItem } from "@/app/lib/editor/state/pending";
import { getState, isTextEntry, pendingSaves, setLeaveGate } from "@/app/lib/editor/state/store";

// ---- drafts: a field being typed in ----

// The text entry that has focus and its value when it got it: changed since means a draft.
let focused: { readonly el: HTMLElement; readonly value: string } | undefined;
const valueOf = (el: HTMLElement) => (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement ? el.value : (el.textContent ?? ""));
const isComposer = (el: Element) => el.id === "agent-input";

function currentDraft(): "rename" | "field" | undefined {
  const el = document.activeElement;
  if (!(el instanceof HTMLElement) || !isTextEntry(el) || isComposer(el)) return undefined;
  // An open rename is a draft as it is (Enter or leaving it saves it).
  if (el.classList.contains("renaming")) return "rename";
  if (focused?.el === el && valueOf(el) !== focused.value) return "field";
  return undefined;
}

/** Saves the field being typed in: a rename takes Enter, the rest save when they're left. */
function commitDraft() {
  const el = document.activeElement;
  if (!(el instanceof HTMLElement) || !isTextEntry(el) || isComposer(el)) return;
  if (el.classList.contains("renaming")) el.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
  else el.blur();
}

/** Drops it: a rename takes Esc; any other field keeps its typing unsaved (nothing is sent). */
function dropDraft() {
  const el = document.activeElement;
  if (el instanceof HTMLElement && el.classList.contains("renaming")) el.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
}

// ---- what's pending now ----

export function pendingNow(reason: LeaveReason): Array<PendingItem> {
  const s = getState();
  const dir = s.snapshot?.dir;
  const recorder = getRecorder().phase;
  return pendingWork({
    reason,
    saving: pendingSaves() + Number(!!s.biliupDirty),
    draft: currentDraft(),
    message: s.snapshot !== null && s.agentDraft.trim() !== "",
    recorder,
    uploading: s.uploading ? s.uploading.total - s.uploading.done : 0,
    imports: s.jobs.filter((j) => j.projectDir === dir && (j.phase === "queued" || j.phase === "probing" || j.phase === "adding")).length,
  });
}

// ---- the dialog's state ----

interface Asking {
  readonly reason: LeaveReason;
  readonly waiting: boolean;
  readonly answer: (go: boolean) => void;
}
let asking: Asking | undefined;
const listeners = new Set<() => void>();
const setAsking = (next: Asking | undefined) => {
  asking = next;
  for (const l of listeners) l();
};
const useAsking = () =>
  useSyncExternalStore(
    (l) => (listeners.add(l), () => void listeners.delete(l)),
    () => asking,
  );

/**
 * Whether to go ahead with leaving: at once when nothing is pending, else the user's choice. A
 * second ask while the dialog is up is answered with the same choice.
 */
export function confirmLeave(reason: LeaveReason): Promise<boolean> {
  if (!pendingNow(reason).length) return Promise.resolve(true);
  if (asking) {
    const first = asking;
    return new Promise((resolve) => {
      setAsking({ ...first, answer: (go) => (first.answer(go), resolve(go)) });
    });
  }
  return new Promise((resolve) => setAsking({ reason, waiting: false, answer: (go) => (setAsking(undefined), resolve(go)) }));
}

/** Save / Wait: keep what can be kept, then go once nothing that can finish is left. */
function keep(items: ReadonlyArray<PendingItem>) {
  if (!asking) return;
  for (const item of items) {
    if (item.id === "draft") commitDraft();
    else if (item.id === "take" && item.how === "save") void stopTake();
  }
  setAsking({ ...asking, waiting: true });
}

/** Discard / Stop: stop what runs, drop the rest, and go. */
function drop(items: ReadonlyArray<PendingItem>) {
  const now = asking;
  if (!now) return;
  for (const item of items) {
    if (item.id === "draft") dropDraft();
    else if (item.stop === "take") cancelTake();
  }
  now.answer(true);
}

export function LeaveDialog() {
  useEditorLanguage();
  const now = useAsking();
  const [, tick] = useState(0);
  // What's pending changes on its own (a save lands, the turn ends): look again often.
  useEffect(() => {
    if (!now) return;
    const timer = setInterval(() => tick((n) => n + 1), 150);
    return () => clearInterval(timer);
  }, [now]);
  const items = now ? pendingNow(now.reason) : [];
  const left = items.filter((i) => i.how !== "none");
  // Waiting and nothing left that can finish: go (what can't be kept goes with it).
  useEffect(() => {
    if (now?.waiting && !left.length) now.answer(true);
  });
  useEffect(() => {
    if (!now) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      now.answer(false);
    };
    editorEvents.addEventListener("keydown", onKey, true);
    return () => editorEvents.removeEventListener("keydown", onKey, true);
  }, [now]);
  if (!now) return null;
  const words = leaveWords(now.reason, items);
  return (
    <div className="modal-layer leave-layer" data-testid="leave-dialog" data-reason={now.reason} data-waiting={now.waiting ? "true" : undefined}>
      {/* Pressing a button mustn't take focus from the field being typed in: leaving it would save it. */}
      <div className="modal confirm-modal leave" role="alertdialog" aria-label={editorText(words.title)} onMouseDown={(e) => e.preventDefault()}>
        <div className="confirm-title">{editorText(words.title)}</div>
        <div className="muted small">{now.waiting ? editorText("Waiting for:") : editorText("Not finished yet:")}</div>
        <ul className="leave-list" data-testid="leave-items">
          {items.map((i) => (
            <li key={i.id} data-testid="leave-item" data-id={i.id}>
              {editorText(i.text)}
            </li>
          ))}
        </ul>
        <div className="confirm-actions">
          <button className="btn ghost" data-testid="leave-cancel" onClick={() => now.answer(false)}>
            {editorText("Cancel")}</button>
          <button className="btn" data-testid="leave-drop" onClick={() => drop(items)}>
            {words.drop}
          </button>
          {words.keep && !now.waiting && (
            <button className="btn primary" data-testid="leave-keep" onClick={() => keep(items)}>
              {words.keep}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

/** Registers draft tracking only for this mounted editor; dispose removes every listener. */
export function installLeave() {
 setLeaveGate(confirmLeave);
 const element = editorRoot();
 const track = (event: FocusEvent) => { const el = event.target; focused = el instanceof HTMLElement && isTextEntry(el) ? { el, value: valueOf(el) } : undefined; };
 element.addEventListener('focusin', track, true);
 return () => { element.removeEventListener('focusin', track, true); focused = undefined; asking?.answer(false); setLeaveGate(() => Promise.resolve(true)); };
}
