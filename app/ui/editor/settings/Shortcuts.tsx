import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { editorEvents } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { Lock, Plus, RotateCcw, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

import {
  ACTIONS,
  displayKey,
  actionDef,
  assignKey,
  bindingsOf,
  checkKey,
  choosePreset,
  clearAction,
  conflicts,
  isActionId,
  keymap,
  keyOf,
  presetBindings,
  PRESETS,
  removeKey,
  resetAction,
  setRecordingKey,
  useBindings,
  useKeymap,
  type ActionDef,
  type ActionId,
  type Check,
  type PresetId,
} from "@/app/ui/editor/workspace/keymap";
import { actions } from "@/app/lib/editor/state/store";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import { isTextComposition } from '@/app/lib/editor/keyboard';

const GROUPS = [...new Set(ACTIONS.map((a) => a.group))];
const does = (id: ActionId) => actionDef(id).does;

/** The shortcuts table (G3) for the sheet: every row with its current keys, grouped. */
export function ShortcutList() {
  useEditorLanguage();
  const bindings = useBindings();
  return (
    <div className="shortcut-list" data-testid="shortcut-list">
      {GROUPS.map((g) => (
        <div key={editorText(g)} className="shortcut-group">
          <div className="group-label">{editorText(g)}</div>
          {ACTIONS.filter((a) => a.group === g).map((a) => {
            const keys = isActionId(a.id) ? bindings[a.id] : a.keys;
            return (
              <div key={a.id} className="shortcut-row" data-testid="shortcut-row" data-action={a.id}>
                <span className="shortcut-does">{editorText(a.does)}</span>
                <span className="shortcut-keys">{keys.length ? keys.map((k) => <kbd key={k}>{displayKey(k)}</kbd>) : <span className="shortcut-none">{editorText("None")}</span>}</span>
              </div>
            );
          })}
        </div>
      ))}
    </div>
  );
}

/** Where a new key is going: in place of the key at `slot`, or one more. */
type Recording = { readonly action: ActionId; readonly slot?: number };
/** A key that ran into another action or the menu, waiting for the user's choice. */
type Pending = Recording & { readonly key: string; readonly check: Exclude<Check, { kind: "ok" } | { kind: "same" }> };

/**
 * Settings › Shortcuts (#78, D257): a preset, then every action with its keys. A key clicked (or
 * +) listens for a new one; Esc or a click elsewhere stops listening. A key another action has asks
 * to swap or replace; a menu key is refused. Each row clears or goes back to the preset's keys.
 */
export function ShortcutSettings() {
  useEditorLanguage();
  const state = useKeymap();
  const bindings = bindingsOf(state);
  const base = presetBindings(state.preset);
  const [recording, setRecording] = useState<Recording | undefined>(undefined);
  const [pending, setPending] = useState<Pending | undefined>(undefined);
  const listening = useRef<HTMLDivElement>(null);
  const clashes = conflicts(bindings);
  const changes = Object.keys(state.overrides).length;
  const customised = changes > 0;
  // D273: a preset (or Reset all) replaces the user's own changes, so it asks first when there are any.
  const [confirm, setConfirm] = useState<{ readonly preset: PresetId; readonly reset: boolean } | undefined>(undefined);
  const cancelConfirm = useCallback(() => setConfirm(undefined), []);
  const choose = (preset: PresetId, reset: boolean) => {
    setPending(undefined);
    setRecording(undefined);
    if (customised) setConfirm({ preset, reset });
    else keymap.set(choosePreset(preset));
  };

  useEffect(() => {
    if (!recording) return;
    setRecordingKey(true);
    const onKey = (e: KeyboardEvent) => {
      if (isTextComposition(e)) return;
      // Every key is ours while listening: nothing else (the editor, the menu's keys, Settings' Esc) sees it.
      e.preventDefault();
      e.stopImmediatePropagation();
      if (e.repeat) return;
      if (e.key === "Escape" && !e.metaKey && !e.altKey && !e.ctrlKey && !e.shiftKey) return setRecording(undefined);
      const key = keyOf(e);
      if (!key) return;
      const now = keymap.get();
      const check = checkKey(bindingsOf(now), recording.action, key);
      setRecording(undefined);
      if (check.kind === "ok") keymap.set(assignKey(now, recording.action, key, recording.slot));
      else if (check.kind !== "same") setPending({ ...recording, key, check });
    };
    const onUp = (e: KeyboardEvent) => e.stopImmediatePropagation();
    const onDown = (e: PointerEvent) => {
      if (!listening.current?.contains(e.target as Node | null)) setRecording(undefined);
    };
    editorEvents.addEventListener("keydown", onKey, true);
    editorEvents.addEventListener("keyup", onUp, true);
    window.addEventListener("pointerdown", onDown, true);
    return () => {
      setRecordingKey(false);
      editorEvents.removeEventListener("keydown", onKey, true);
      editorEvents.removeEventListener("keyup", onUp, true);
      window.removeEventListener("pointerdown", onDown, true);
    };
  }, [recording]);

  const record = (action: ActionId, slot?: number) => (setPending(undefined), setRecording({ action, slot }));
  const resolve = (how: "swap" | "replace") => {
    if (!pending) return;
    keymap.set(assignKey(keymap.get(), pending.action, pending.key, pending.slot, how));
    setPending(undefined);
  };

  const row = (a: ActionDef) => {
    if (!isActionId(a.id)) {
      return (
        <div key={a.id} className="shortcut-row fixed" data-testid="shortcut-row" data-action={a.id}>
          <span className="shortcut-does">{editorText(a.does)}</span>
          <span className="shortcut-keys">
            {a.keys.map((k) => (
              <kbd key={k}>{displayKey(k)}</kbd>
            ))}
            <span className="shortcut-lock" title={a.fixed === "menu" ? editorText("Fixed shortcut") : a.fixed === "pointer" ? editorText("A mouse or trackpad gesture") : editorText("Only in the full-screen preview")}>
              <Lock />
            </span>
          </span>
        </div>
      );
    }
    const id = a.id;
    const keys = bindings[id];
    const live = recording?.action === id;
    const changed = state.overrides[id] !== undefined;
    const clash = clashes.some((c) => c.actions.includes(id));
    return (
      <div key={id} className={`shortcut-edit${live ? " live" : ""}`} data-testid="shortcut-row" data-action={id} ref={live ? listening : undefined}>
        <div className="shortcut-row">
          <span className="shortcut-does">
            {editorText(a.does)}
            {changed && <span className="shortcut-changed" title={editorText("Changed from {0}", { 0: base[id].map(displayKey).join(", ") || editorText("no key") })} />}
          </span>
          <span className="shortcut-keys">
            {keys.map((k, i) =>
              live && recording?.slot === i ? (
                <span key={k} className="shortcut-listening" data-testid="shortcut-recording">
                  {editorText("Press a key…")}</span>
              ) : (
                <span key={k} className={`shortcut-chip${clash && clashes.some((c) => c.key === k) ? " clash" : ""}`}>
                  <button className="shortcut-key" data-testid="shortcut-key" data-key={k} title={editorText("Click, then press a new key")} onClick={() => record(id, i)}>
                    {k}
                  </button>
                  <Tooltip label={editorText("Remove {0}", { 0: k })}>
                    <button className="shortcut-remove" data-testid="shortcut-remove" aria-label={editorText("Remove {0}", { 0: k })} onClick={() => (setPending(undefined), keymap.set(removeKey(keymap.get(), id, i)))}>
                      <X />
                    </button>
                  </Tooltip>
                </span>
              ),
            )}
            {live && recording?.slot === undefined ? (
              <span className="shortcut-listening" data-testid="shortcut-recording">
                {editorText("Press a key…")}</span>
            ) : (
              <Tooltip label={keys.length ? editorText("Add another key") : editorText("Set a key")}>
                <button className="icon-btn shortcut-add" data-testid="shortcut-add" aria-label={editorText("Add a key")} onClick={() => record(id)}>
                  <Plus />
                </button>
              </Tooltip>
            )}
            <button className="link-btn shortcut-clear" data-testid="shortcut-clear" disabled={!keys.length} onClick={() => (setPending(undefined), keymap.set(clearAction(keymap.get(), id)))}>
              {editorText("Clear")}</button>
            <Tooltip label={editorText("Back to {0}", { 0: base[id].map(displayKey).join(", ") || editorText("no key") })}>
              <button className="icon-btn shortcut-reset" data-testid="shortcut-reset" disabled={!changed} aria-label={editorText("Reset")} onClick={() => (setPending(undefined), keymap.set(resetAction(keymap.get(), id)))}>
                <RotateCcw />
              </button>
            </Tooltip>
          </span>
        </div>
        {live && <div className="shortcut-hint">{editorText("Press the new key (Esc cancels).")}</div>}
        {pending?.action === id && (
          <div className="shortcut-conflict" data-testid="shortcut-conflict" role="alert">
            {pending.check.kind === "reserved" ? (
              <>
                <span className="grow">
                  <kbd>{displayKey(pending.key)}</kbd> {editorText(" is ")}{editorText(pending.check.by)}{editorText(", a fixed shortcut. Pick another key.")}</span>
                <button className="btn" onClick={() => record(id, pending.slot)}>
                  {editorText("Try again")}</button>
                <button className="btn" onClick={() => setPending(undefined)}>
                  {editorText("Cancel")}</button>
              </>
            ) : (
              <>
                <span className="grow">
                  <kbd>{displayKey(pending.key)}</kbd> {editorText(" is already ")}{does(pending.check.by)}.
                </span>
                {pending.slot !== undefined && keys[pending.slot] !== undefined && (
                  <button className="btn" data-testid="shortcut-swap" title={editorText("{0} gets {1}", { 0: does(pending.check.by), 1: displayKey(keys[pending.slot]) })} onClick={() => resolve("swap")}>
                    {editorText("Swap")}</button>
                )}
                <button className="btn primary" data-testid="shortcut-replace" title={editorText("{0} loses {1}", { 0: does(pending.check.by), 1: pending.key })} onClick={() => resolve("replace")}>
                  {editorText("Replace")}</button>
                <button className="btn" data-testid="shortcut-cancel" onClick={() => setPending(undefined)}>
                  {editorText("Cancel")}</button>
              </>
            )}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="shortcut-settings" data-testid="shortcut-settings">
      <div className="shortcut-head">
        <label className="shortcut-preset">
          {editorText("Preset")}<select data-testid="shortcut-preset" value={state.preset} onChange={(e) => choose(e.target.value as PresetId, false)}>
            {PRESETS.map((p) => (
              <option key={p.id} value={p.id}>
                {editorText(p.label)}
              </option>
            ))}
          </select>
        </label>
        {customised && <span className="muted small" data-testid="shortcut-customised">{editorText("Customised")}</span>}
        <span className="grow" />
        <button className="btn" data-testid="shortcuts-reset-all" disabled={!customised} title={editorText("Drop your changes and use the preset's keys")} onClick={() => choose(state.preset, true)}>
          {editorText("Reset all")}</button>
      </div>
      {clashes.length > 0 && (
        <div className="shortcut-clashes" data-testid="shortcut-conflicts" role="status">
          {clashes.map((c) => (
            <div key={c.key}>
              <kbd>{displayKey(c.key)}</kbd> {editorText(" is used by ")}{[...c.actions.map(does), ...(c.reserved ? [editorText(c.reserved)] : [])].join(" / ")}.
            </div>
          ))}
        </div>
      )}
      <div className="shortcut-list" data-testid="shortcut-list">
        {GROUPS.map((g) => (
          <div key={editorText(g)} className="shortcut-group">
            <div className="group-label">{editorText(g)}</div>
            {ACTIONS.filter((a) => a.group === g).map(row)}
          </div>
        ))}
      </div>
      {confirm && (
        <PresetConfirm
          label={PRESETS.find((p) => p.id === confirm.preset)?.label ?? confirm.preset}
          reset={confirm.reset}
          changes={changes}
          onCancel={cancelConfirm}
          onReplace={() => {
            setConfirm(undefined);
            keymap.set(choosePreset(confirm.preset));
          }}
        />
      )}
    </div>
  );
}

/**
 * D273: in the app (like Remove media's, F8), not a system dialog. Esc or a click outside cancels;
 * while it's open, Esc is its own (Settings under it stays open).
 */
function PresetConfirm({ label, reset, changes, onCancel, onReplace }: { label: string; reset: boolean; changes: number; onCancel: () => void; onReplace: () => void }) {
  useEditorLanguage();
  useEffect(() => {
    setRecordingKey(true);
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || e.defaultPrevented || isTextComposition(e)) return;
      e.preventDefault();
      e.stopImmediatePropagation();
      onCancel();
    };
    editorEvents.addEventListener("keydown", onKey, true);
    return () => {
      setRecordingKey(false);
      editorEvents.removeEventListener("keydown", onKey, true);
    };
  }, [onCancel]);
  const yours = changes === 1 ? "the 1 shortcut you changed" : `the ${changes} shortcuts you changed`;
  return (
    <div className="modal-layer shortcut-confirm-layer" data-testid="shortcut-preset-confirm" onPointerDown={(e) => e.target === e.currentTarget && onCancel()}>
      <div className="modal confirm-modal" role="alertdialog" aria-label={reset ? editorText("Reset all shortcuts") : editorText("Use {0}", { 0: label })}>
        <div className="confirm-title">{reset ? editorText("Reset all shortcuts?") : editorText("Switch to {0}?", { 0: label })}</div>
        <div className="confirm-text">{reset ? editorText("{0}'s keys replace {1}.", { 0: label, 1: yours }) : editorText("Its keys replace {0}.", { 0: yours })}</div>
        <div className="confirm-actions">
          <button className="btn" data-testid="shortcut-preset-cancel" onClick={onCancel}>
            {editorText("Cancel")}</button>
          <button className="btn primary" data-testid="shortcut-preset-ok" autoFocus onClick={onReplace}>
            {reset ? editorText("Reset all") : editorText("Use {0}", { 0: label })}
          </button>
        </div>
      </div>
    </div>
  );
}

/** The Keyboard shortcuts sheet: Help › Keyboard Shortcuts or ?; Esc or a click outside closes it. */
export function ShortcutsSheet() {
  useEditorLanguage();
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || e.defaultPrevented || isTextComposition(e)) return;
      e.preventDefault();
      e.stopImmediatePropagation();
      actions.closeShortcuts();
    };
    editorEvents.addEventListener("keydown", onKey, true);
    return () => editorEvents.removeEventListener("keydown", onKey, true);
  }, []);
  return (
    <div className="modal-layer" data-testid="shortcuts-sheet" onPointerDown={(e) => e.target === e.currentTarget && actions.closeShortcuts()}>
      <div className="modal sheet">
        <div className="sheet-head">
          <span className="grow">{editorText("Keyboard shortcuts")}</span>
          <Tooltip label={editorText("Close")} keys={["Esc"]}>
            <button className="icon-btn" data-testid="shortcuts-close" onClick={actions.closeShortcuts}>
              <X />
            </button>
          </Tooltip>
        </div>
        <ShortcutList />
      </div>
    </div>
  );
}
