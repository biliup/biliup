// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { actions } from "@/app/lib/editor/state/store/actions";
import { online, state } from "@/app/lib/editor/state/store/core";
import { justClosedProject } from "@/app/lib/editor/state/store/actions/project";
import { panels } from "@/app/ui/editor/workspace/workspaceLayout";
import { editorDialogOpen } from '@/app/lib/editor/environment';
import { keyOf } from '@/app/ui/editor/workspace/keymap';

type MenuCommand =
  | { readonly type: "new-project" | "open-project" | "close-project" | "undo" | "redo" | "open-setup" | "open-settings" | "shortcuts" | "export" }
  | { readonly type: "open-recent"; readonly dir: string }
  | { readonly type: "toggle-panel"; readonly panel: "lib" | "side" | "bottom" }
  | { readonly type: "focus-mode" };

export function runCommand(command: MenuCommand) {
  // #76 (D256): the editor's layout is local UI too (only with a project open).
  if (command.type === "toggle-panel") return void (state.snapshot && panels.toggle(command.panel));
  if (command.type === "focus-mode") return void (state.snapshot && panels.focus());
  // Settings and the shortcuts sheet are local UI: they open while the server is away too.
  if (command.type === "open-settings") return actions.openSettings();
  if (command.type === "shortcuts") return actions.openShortcuts();
  // Everything below talks to the server; while it's away the buttons are disabled too.
  if (!online()) return;
  if (command.type === "open-setup") return actions.openSetup();
  if (command.type === "export") return actions.openExport();
  switch (command.type) {
    case "new-project":
      {const query=new URLSearchParams(window.location.search);query.set('new','1');window.history.pushState(null,'',`/workbench?${query}`);return;}
    case "open-project":
      return void actions.openWithDialog();
    case "open-recent":
      return void actions.openProject(command.dir);
    case "close-project":
      if (new URLSearchParams(window.location.search).has('new')) {
        const query = new URLSearchParams(window.location.search);
        query.delete('new');
        window.history.replaceState(null, '', `/workbench?${query}`);
        return;
      }
      return void actions.closeProject();
    case "undo":
    case "redo":
      return void actions[command.type]();
  }
}

const TEXT_INPUT_TYPES = new Set(["text", "search", "number", "email", "url", "tel", "password"]);

export const isTextEntry = (el: EventTarget | null) =>
  el instanceof HTMLTextAreaElement || (el instanceof HTMLInputElement && TEXT_INPUT_TYPES.has(el.type)) || (el instanceof HTMLElement && el.isContentEditable);

// Fixed commands share the same key normalization as configurable timeline actions.
export function onKeyDown(event: KeyboardEvent) {
  if (event.defaultPrevented || state.exportOpen || state.settingsOpen || state.shortcutsOpen || editorDialogOpen()
    || (event.target instanceof Element && event.target.closest('[role=menu], [role=listbox]'))) return;
  const pressed = keyOf(event);
  const prefix = event.shiftKey ? '⇧⌘' : '⌘';
  if (!pressed?.startsWith(prefix)) return;
  const key = pressed.slice(prefix.length).toLowerCase();
  // The browser owns text undo, including its input events and IME undo history.
  if (key === 'z' && isTextEntry(event.target)) return;
  // D243: with no project ⌘W is the menu's native Close (the window, and the app). A ⌘W held
  // down, or pressed again, just as a project closes would reach it: kept here, so closing a
  // project never quits Biliup.
  if (key === "w" && !event.shiftKey && !state.snapshot && (event.repeat || justClosedProject())) return void event.preventDefault();
  const command: MenuCommand | undefined =
    key === "z"
      ? { type: event.shiftKey ? "redo" : "undo" }
      : event.shiftKey
        ? layoutCommand(key)
        : key === ","
          ? { type: "open-settings" }
          : key === "n"
          ? { type: "new-project" }
          : key === "e" && state.snapshot
          ? { type: "export" }
          : key === "o"
            ? { type: "open-project" }
            : key === "w" && state.snapshot
              ? { type: "close-project" }
              : undefined;
  if (!command) return;
  event.preventDefault();
  if (event.repeat && command.type !== 'undo' && command.type !== 'redo') return;
  runCommand(command);
}

/** #76 (D256): ⇧⌘L, ⇧⌘I, ⇧⌘T hide or show the left panel, the right panel, the timeline; ⇧⌘F is focus mode. */
function layoutCommand(key: string): MenuCommand | undefined {
  if (!state.snapshot) return undefined;
  if (key === "l") return { type: "toggle-panel", panel: "lib" };
  if (key === "i") return { type: "toggle-panel", panel: "side" };
  if (key === "t") return { type: "toggle-panel", panel: "bottom" };
  if (key === "f") return { type: "focus-mode" };
  return undefined;
}
