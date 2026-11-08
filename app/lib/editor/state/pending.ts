// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Browser-owned work that can be lost when leaving the editor.
// Server jobs keep running independently of this page.

export type LeaveReason = "quit" | "window" | "close-project" | "open-project" | "new-project";

export interface PendingInput {
  readonly reason: LeaveReason;
  /** Project edits sent to the server and not answered yet (or queued behind one). */
  readonly saving: number;
  /** A field being typed in: an open rename, or another value not saved until the field is left. */
  readonly draft: "rename" | "field" | undefined;
  /** Text in the agent's composer that wasn't sent. */
  readonly message: boolean;
  /** A recording captured in this browser. */
  readonly recorder: "idle" | "countdown" | "recording" | "saving";
  /** Files still being uploaded from this browser. */
  readonly uploading: number;
  /** Media imports of the open project still adding media. */
  readonly imports: number;
}

/**
 * How the dialog's first button deals with an item: `save` commits or stops-and-keeps it, `wait`
 * lets it finish, `none` can't be kept (it's lost on leaving). The second button stops or drops
 * every item (`stop` names the way, if there is one).
 */
export interface PendingItem {
  readonly id: string;
  readonly text: string;
  readonly how: "save" | "wait" | "none";
  readonly stop?: "take" | "rename";
}

const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;

/** What's pending for this way of leaving, in the order the dialog lists it. */
export function pendingWork(input: PendingInput): Array<PendingItem> {
  const out: Array<PendingItem> = [];
  if (input.draft === "rename") out.push({ id: "draft", text: "A name being edited", how: "save", stop: "rename" });
  else if (input.draft === "field") out.push({ id: "draft", text: "A value being typed", how: "save" });
  if (input.saving > 0) out.push({ id: "saving", text: input.saving === 1 ? "The last change is being saved" : `${plural(input.saving, "change")} being saved`, how: "wait" });
  if (input.recorder === "recording" || input.recorder === "countdown") out.push({ id: "take", text: "A recording in this browser", how: "save", stop: "take" });
  else if (input.recorder === "saving") out.push({ id: "take", text: "A recording being saved", how: "wait" });
  if (input.uploading > 0) out.push({ id: "uploading", text: `Uploading ${plural(input.uploading, "file")}`, how: "wait" });
  if (input.imports > 0) out.push({ id: "imports", text: "Media being imported", how: "wait" });
  if (input.message) out.push({ id: "message", text: "An unsent message to the agent (not kept)", how: "none" });
  return out;
}

/** The dialog's words for a way of leaving and what's pending. */
export function leaveWords(reason: LeaveReason, items: ReadonlyArray<PendingItem>) {
  const title =
    reason === "quit"
      ? "Leave the editor?"
      : reason === "window"
        ? "Leave the editor?"
        : reason === "close-project"
          ? "Close the project?"
          : "Leave this project?";
  const saves = items.some((i) => i.how === "save");
  const waits = items.some((i) => i.how === "wait");
  // Something running (it stops) and something unsaved (it's dropped).
  const running = items.some((i) => (i.how === "wait" && i.id !== "saving") || i.stop === "take");
  const unsaved = items.some((i) => i.id === "draft" || i.id === "saving" || i.how === "none");
  const keep = saves && waits ? "Save and wait" : saves ? "Save" : waits ? "Wait" : undefined;
  const drop = running && unsaved ? "Discard and stop" : running ? "Stop" : "Discard";
  return { title, keep, drop };
}
