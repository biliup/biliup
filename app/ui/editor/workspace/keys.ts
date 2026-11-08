import { editorDialogOpen, editorEvents, focusEditor } from "@/app/lib/editor/environment";
import { editorRoot } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The editor's keyboard shortcuts (G3, D118). The keymap (editor/keymap.ts, #78 D257: presets and
// the user's own keys) is what the Keyboard shortcuts sheet and Settings show; useEditorKeys is
// what they do. They never fire while a text field, a list or a slider has focus, or while a
// dialog is open. ⌘Z/⇧⌘Z and the menu's keys (⌘N, ⌘O, ⌘E, ⌘W, ⌘,) live in the store.
import { useEffect } from "react";

import { cutSelectedWords } from "@/app/ui/editor/panels/TranscriptPanel";
import { addSelectedWordsToChat } from "@/app/ui/editor/panels/transcriptView/cuts";
import { getTranscripts, transcriptStore } from "@/app/ui/editor/panels/transcript";
import { getPlayback, getPreviewPlayback, playback, previewPlayback } from "@/app/ui/editor/preview/playback";
import { actions, getState, isTextEntry, projectOf } from "@/app/lib/editor/state/store";
import { installFocusRules, isTyping } from "@/app/ui/editor/workspace/focus";
import { copyItems, nextEditPoint, planDeleteMany, planPaste, type Clip } from "@/app/ui/editor/timeline/clipboard";
import {currentEditorStore} from '@/app/lib/editor/state/store/core';
import { timelineCommand } from '@/app/lib/editor/state/store/edits';
import { nearbyKey } from "@/app/ui/editor/timeline/clip";
import { findItem } from "@/app/ui/editor/timeline/edit";
import { toolKeys, type Tool } from "@/app/ui/editor/timeline/tools";
import { actionFor, keymap, keyOf, keysOf, type ActionId, type ShortcutId } from "@/app/ui/editor/workspace/keymap";
import { isTextComposition } from '@/app/lib/editor/keyboard';

/** A row's current keys (issue #78, D257: the keymap in editor/keymap.ts), for tooltips and lists. */
export const shortcutKeys = (id: ShortcutId): ReadonlyArray<string> => keysOf(id);
export type { ShortcutId };

// Keys belong to the control while typing, picking from a list or moving a slider.
export const isEditable = (el: EventTarget | null) => isTextEntry(el) || el instanceof HTMLSelectElement || (el instanceof HTMLInputElement && el.type === "range");

// Copied source IDs and frames belong to their project session, not the next opened project.
// A session's clipboard is released with its store; no editor data enters the system clipboard.
const clipboards=new WeakMap<object,ReadonlyArray<Clip>>();
const EMPTY_CLIPS:ReadonlyArray<Clip>=[];
export const clipboardItems = () => {const owner=currentEditorStore();return owner?clipboards.get(owner)??EMPTY_CLIPS:EMPTY_CLIPS};

const selection = () => {
  const { selected, also } = getPlayback();
  return selected === undefined ? [] : [selected, ...also];
};

/** ⌘C: copy the selection; false when nothing copyable is selected. */
export function copySelection(ids: ReadonlyArray<string> = selection()): boolean {
  const snapshot = getState().snapshot;
  const owner=currentEditorStore();
  if (!snapshot||!owner) return false;
  const copied = copyItems(projectOf(snapshot), ids);
  if (!copied.length) return false;
  clipboards.set(owner,copied);
  return true;
}

/** ⌘V: paste what was copied at the playhead; the pasted items end up selected. */
export async function paste(at = getPlayback().frame) {
  const snapshot = getState().snapshot;
  if (!snapshot) return;
  const plan = planPaste(projectOf(snapshot), clipboardItems(), at);
  return timelineCommand(plan, () => playback.selectMany("ids" in plan ? plan.ids : []));
}

/** Cut copies exactly the items removed, and only changes the clipboard after a successful save. */
export async function cutSelection(ids: ReadonlyArray<string> = selection()) {
  const snapshot = getState().snapshot, owner = currentEditorStore();
  if (!snapshot || !owner) return false;
  const project = projectOf(snapshot), copied = copyItems(project, ids);
  if (!copied.length) return false;
  const previous = clipboards.get(owner);
  const changed = await timelineCommand(planDeleteMany(project, copied.map(clip => clip.item.id)), () => playback.select(undefined));
  // A later Copy wins over a cut that was still waiting for the server.
  if (changed && currentEditorStore() === owner && clipboards.get(owner) === previous) clipboards.set(owner, copied);
  return changed;
}

/** Delete the focused key/transition, or all selected items and their linked/grouped parts. */
export const deleteSelection = (ids?: ReadonlyArray<string>) => actions.deleteSelected(ids);

/** ⌘D: a copy of the selection right after it (on main: after the clip; elsewhere: where it ends). */
export async function duplicate(ids: ReadonlyArray<string> = selection()) {
  const snapshot = getState().snapshot;
  const project = snapshot && projectOf(snapshot);
  if (!snapshot || !project || !ids.length) return;
  const copied = copyItems(project, ids);
  if (!copied.length) return;
  const placed = copied.map(clip=>findItem(project,clip.item.id)).filter(p=>p!==undefined);
  const end = Math.max(...placed.map((p) => p.start + p.item.duration));
  const plan = planPaste(project, copied, end);
  return timelineCommand(plan, () => playback.selectMany("ids" in plan ? plan.ids : []));
}

// Issue #49: the Agent tab's conversation is text to read and copy; a selection in it beats the
// timeline's ⌘C / ⌘X (which would otherwise preventDefault and keep the menu's Copy from running).
const inAgentThread = (node: Node | null) => (node instanceof Element ? node : (node?.parentElement ?? null))?.closest("[data-testid=agent-thread]") != null;
export function agentTextSelected(): boolean {
  const sel = document.getSelection();
  return !!sel && !sel.isCollapsed && sel.toString().trim() !== "" && inAgentThread(sel.anchorNode);
}

const TOOL_OF: Partial<Record<ActionId, Tool>> = { toolSelect: "select", toolBlade: "blade", toolHand: "hand" };

export function useEditorKeys(enabled=true) {
  useEffect(() => {
    if(!enabled)return;
    const s = () => getState();
    const uninstall = installFocusRules();
    const dialog = () => s().exportOpen || s().settingsOpen || s().shortcutsOpen || editorDialogOpen();
    // #48: Space plays unless a text field is really being typed in (editor/focus.ts); a list, a
    // slider or a field focused for the user doesn't take it.
    const space = (e: KeyboardEvent) => e.code === "Space" && !e.metaKey && !e.altKey && !e.ctrlKey;
    // D264: the add-to-chat chord (⌘L) too: Add to chat leaves the caret in the composer, and the
    // timeline keeps that focus, so only a field the user is really typing in takes it.
    const chatChord = (e: KeyboardEvent) => {
      if (!e.metaKey && !e.ctrlKey && !e.altKey) return false;
      const pressed = keyOf(e);
      return pressed !== undefined && actionFor(keymap.bindings(), pressed) === "addToChat";
    };
    const blocked = (e: KeyboardEvent) => isTextComposition(e) || (space(e) || chatChord(e) ? isTyping(e.target) : isEditable(e.target))
      || (e.target instanceof Element && !!e.target.closest('[role=menu], [role=listbox]')) || dialog();
    const onKeyDown = (e: KeyboardEvent) => {
      // A key the menu's handler (store/keys.ts, registered first) already took isn't run twice.
      if (e.defaultPrevented || blocked(e)) return;
      const done = () => e.preventDefault();
      const bottom = getPlayback().bottom;
      // U11 (D177): the Transcript is a rail tab next to the timeline. Words selected in it (which
      // clears the timeline's selection, and the other way round) are what ⌫ cuts.
      const words = getTranscripts().anchor !== undefined && editorRoot().querySelector("[data-testid=transcript-panel]") !== null;
      // ⌘A, ⌘C, ⌘X, ⌘V are the menu's (fixed in the keymap): text fields share them.
      const pressed = keyOf(e);
      const key = pressed?.slice(1).toLowerCase();
      if (pressed && ['⌘A', '⌘C', '⌘X', '⌘V'].includes(pressed)) {
        // With words selected and in the mixer they keep their usual meaning.
        if (bottom !== "timeline" || words) return;
        // Issue #49: text selected in the Agent tab (a reply, a step's output) copies as text, even
        // with clips selected; ⌘A there selects the conversation, not the timeline.
        if ((key === "c" || key === "x") && agentTextSelected()) return;
        if (key === "a" && inAgentThread(document.getSelection()?.anchorNode ?? null)) {
          done();
          const thread = editorRoot().querySelector("[data-testid=agent-thread]");
          if (thread) document.getSelection()?.selectAllChildren(thread);
        } else if (key === "a") {
          done();
          const snapshot = s().snapshot;
          const ids = snapshot ? projectOf(snapshot).tracks.flatMap((t) => t.items.filter((i) => i.type !== "captions").map((i) => i.id)) : [];
          playback.selectMany(ids);
        } else if (key === "c") {
          if (copySelection()) done();
        } else if (key === "x") {
          if (selection().length) (done(), e.repeat || void cutSelection());
        } else if (key === "v") {
          if (clipboardItems().length) (done(), e.repeat || void paste());
        }
        return;
      }
      const action = pressed === undefined ? undefined : actionFor(keymap.bindings(), pressed);
      if (!action) return;
      const snapshot = s().snapshot;
      const seekTo = (to: number | null | undefined) => to !== undefined && to !== null && (playback.pause(), playback.seek(to));
      // Issue #57 (D230): the tool keys pick the timeline's tool (held: only while down).
      const tool = TOOL_OF[action];
      if (tool) return (done(), toolKeys.down(tool, e.code, e.repeat));
      switch (action) {
        case "play":
          // Also stops a focused button from being "clicked" by the same key.
          done();
          // A field focused without the user choosing it lets go, so the next keys are shortcuts too.
          if (isEditable(e.target)) focusEditor();
          if (!e.repeat) previewPlayback.toggle();
          return;
        case "shuttleBack":
          return (done(), previewPlayback.shuttle(-1));
        case "pause":
          return (done(), previewPlayback.pause());
        case "shuttleForward":
          return (done(), previewPlayback.shuttle(1));
        case "frameBack":
          return (done(), previewPlayback.step(-1));
        case "frameForward":
          return (done(), previewPlayback.step(1));
        case "secondBack":
          return (done(), previewPlayback.step(-getPreviewPlayback().fps));
        case "secondForward":
          return (done(), previewPlayback.step(getPreviewPlayback().fps));
        case "toStart":
          return (done(), previewPlayback.toStart());
        case "toEnd":
          return (done(), previewPlayback.toEnd());
        case "prevEdit":
        case "nextEdit":
          done();
          return void seekTo(snapshot && nextEditPoint(projectOf(snapshot), getPlayback().frame, action === "prevEdit" ? -1 : 1));
        case "prevKey":
        case "nextKey":
          done();
          return void seekTo(snapshot && nearbyKey(projectOf(snapshot), getPlayback().frame, action === "prevKey" ? -1 : 1, selection()));
        case "markIn":
          return (done(), playback.setMark("in", Math.round(getPlayback().frame)));
        case "markOut":
          return (done(), playback.setMark("out", Math.round(getPlayback().frame)));
        case "addMarker":
          return void (done(), e.repeat || void actions.addMarker());
        case "split":
          return void (done(), e.repeat || void actions.split());
        case "delete":
        case "rippleDelete":
          done();
          // Selected words in the Transcript: ⌫ cuts them (D69). The Mixer (D83) shows no selection,
          // so ⌫ there deletes nothing else.
          if (!e.repeat && (words || bottom !== "mixer"))
            void (words ? cutSelectedWords() : action === "rippleDelete" && getPlayback().cut === undefined && getPlayback().gainKey === undefined ? actions.rippleDelete(selection()) : deleteSelection());
          return;
        case "duplicate":
          if (bottom !== "timeline" || words) return;
          return void (done(), e.repeat || void duplicate());
        case "deselect":
          return void (playback.select(undefined), transcriptStore.clear());
        case "zoomIn":
          return (done(), playback.zoomIn());
        case "zoomOut":
          return (done(), playback.zoomOut());
        case "fit":
          return (done(), playback.zoomFit());
        case "addToChat": {
          // D264 (#77): words picked in the Transcript, else the timeline's selection.
          if (e.repeat) return done();
          if (words) return void (addSelectedWordsToChat() && done());
          const ids = selection();
          if (!ids.length) return;
          done();
          return ids.forEach((id) => actions.addItemToChat(id));
        }
        case "shortcuts":
          return (done(), actions.openShortcuts());
      }
    };
    const onKeyUp = (e: KeyboardEvent) => {
      if (e.code === "Space" && !blocked(e)) e.preventDefault();
      // A held tool key gives the tool back wherever focus went meanwhile.
      toolKeys.up(e.code);
    };
    const onPointerDown = () => toolKeys.pointer();
    const onBlur = () => toolKeys.blur();
    const onFocusOut = () => toolKeys.blur();
    editorEvents.addEventListener("keydown", onKeyDown);
    editorEvents.addEventListener("keyup", onKeyUp);
    editorEvents.addEventListener("focusout", onFocusOut);
    window.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("blur", onBlur);
    return () => {
      uninstall();
      editorEvents.removeEventListener("keydown", onKeyDown);
      editorEvents.removeEventListener("keyup", onKeyUp);
      editorEvents.removeEventListener("focusout", onFocusOut);
      window.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener("blur", onBlur);
      toolKeys.blur();
    };
  }, [enabled]);
}
