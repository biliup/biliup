import { editorRoot } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { ProjectSnapshot } from "@/app/lib/editor/contracts";
import { type CallError, type Result, isTransportError } from "@/app/lib/editor/state/connection";
import { playback, getPlayback } from "@/app/ui/editor/preview/playback";
import { type Plan, draftOf } from "@/app/ui/editor/timeline/edit";
import { type ShaderDef, SHADER, shaderUseOf } from "@/app/lib/editor/core/visual/shaders";
import type { ProjectModel, Planned } from "@/app/lib/editor/state/project";
import { call, refreshRecents, plainError } from "@/app/lib/editor/state/store/server";
import { state, projectOf, set, online, currentEditorStore } from "@/app/lib/editor/state/store/core";
import { applyFromCall } from "@/app/lib/editor/state/store/sync";
import type { EditorStore } from '@/app/lib/editor/project-store';
import { editorText } from '@/app/lib/editor/i18n';

/**
 * Runs a call that changes the project. Success clears the banner. A transport failure is
 * "unknown", not "didn't happen": nothing is rolled back here, the subscription will show
 * what the server did. Returns the error (if any) for callers that show it inline.
 */
// U12 (D178): asked before leaving the project; state/leave.tsx sets it (it
// imports this module, so it registers itself instead of being imported here).
type LeaveGate = (reason: "close-project" | "open-project" | "new-project") => Promise<boolean>;
export let confirmLeave: LeaveGate = () => Promise.resolve(true);
export const setLeaveGate = (gate: LeaveGate) => void (confirmLeave = gate);

// Project changes sent and not answered yet, and item edits queued behind them (U12: leaving waits for them).
type Editing = { inflight: number; queued: number; adding: boolean; chain: Promise<unknown> };
const editing = new WeakMap<EditorStore, Editing>();
function editingFor(owner: EditorStore): Editing {
  let value = editing.get(owner);
  if (!value) { value = { inflight: 0, queued: 0, adding: false, chain: Promise.resolve() }; editing.set(owner, value); }
  return value;
}
export const pendingSaves = () => {
  const owner = currentEditorStore(), work = owner && editing.get(owner);
  return work ? work.inflight + work.queued : 0;
};

export async function change(run: () => Promise<Result<ProjectSnapshot>>, { inline = false } = {}): Promise<CallError | undefined> {
  const owner=currentEditorStore();
  const work = owner && editingFor(owner);
  const projectDir = state.snapshot?.dir;
  if (work) work.inflight++;
  let result: Result<ProjectSnapshot>;
  try {
    result = await run();
  } finally {
    if (work) work.inflight--;
  }
  if (currentEditorStore()!==owner||state.snapshot?.dir !== projectDir) return undefined;
  if (result.ok) {
    const renamed = state.snapshot && projectOf(state.snapshot).name !== projectOf(result.value).name;
    applyFromCall(result.value);
    set({ notice: undefined });
    if (renamed) void refreshRecents(); // the Open Recent menu shows the new name
    return undefined;
  }
  const error = result.error;
  if (!inline) set({ notice: { text: plainError(error), transport: isTransportError(error) } });
  return error;
}

/** Plan against the project on screen, then join the same queue as timeline and inspector edits. */
export async function addPlanned(plan: (project: ProjectModel) => Planned | { readonly problem: string }) {
  const owner = currentEditorStore(), snapshot = state.snapshot;
  if (!owner || !snapshot || !online() || !state.biliupCanEdit) return;
  const planned = plan(projectOf(snapshot));
  const from = getPlayback();
  return timelineCommand(planned, () => {
    if ('problem' in planned) return;
    playback.select(planned.itemId);
    // A seek or playback while saving belongs to the user, not to the completed import.
    if (getPlayback().frame === from.frame && getPlayback().playing === from.playing) {
      playback.pause();
      playback.seek(planned.show ?? planned.start);
      revealItem(planned.itemId);
    }
  });
}

/** Scroll the timeline so an item is in view (after it was added somewhere off screen). */
export function revealItem(itemId: string) {
  const owner = currentEditorStore();
  if (!owner) return;
  const root = editorRoot();
  requestAnimationFrame(() => {
    if (currentEditorStore() !== owner || !root.isConnected) return;
    const el = root.querySelector<HTMLElement>(`[data-testid=timeline-item][data-item="${CSS.escape(itemId)}"]`);
    el?.scrollIntoView({ block: "nearest", inline: "nearest" });
  });
}

/**
 * A manual timeline edit (D52): ops planned against the project at `baseRevision`, sent as one
 * project.edit, so it is validated, undoes in one step and is listed in the agent's next note.
 * If the agent changed the same item after that revision the server refuses it (D6) and the
 * banner says so. Shares the one-at-a-time guard with adds. Returns whether it was applied.
 */
// ---- a number field's scrub (D209): edits become a preview while it drags ----

let previewing = false;
let scrubDraft: { owner:EditorStore; base: ProjectModel; project: ProjectModel } | undefined;

/** Shows a plan in the preview (and the mix) without sending it. */
function previewPlan(plan: Plan | { readonly problem: string }) {
  const snapshot = state.snapshot;
  const owner=currentEditorStore();
  if (!owner || !snapshot || !online() || !state.biliupCanEdit || "problem" in plan || !plan.ops.length) return;
  const base = projectOf(snapshot);
  const result = draftOf(base, plan.ops);
  if (!("project" in result)) return;
  scrubDraft = { owner, base, project: result.project };
  playback.setDraft(scrubDraft);
}

/**
 * Runs `fn` with every edit it makes (itemEdit, timelineEdit) shown in the preview instead of sent:
 * a number field calls its own commit this way while it's dragged, then once for real on release
 * (one undoable edit).
 */
export function previewEdits(fn: () => void) {
  previewing = true;
  try {
    fn();
  } finally {
    previewing = false;
  }
}

/** Drops a scrub's preview, now or once the edit it ended with has come back (a new revision, at most 2 s). */
export function endPreview(afterEdit: boolean,owner=currentEditorStore()) {
  const draft = scrubDraft;
  if(!draft||draft.owner!==owner)return;
  scrubDraft = undefined;
  const drop = () => {
    if (getPlayback().draft === draft) playback.setDraft(undefined);
  };
  if (!afterEdit) return drop();
  const revision = draft.owner.getSnapshot().snapshot?.revision;
  const timer = setTimeout(() => (stop(), drop()), 2000);
  const stop = draft.owner.subscribe(() => {
    if (currentEditorStore()!==draft.owner || draft.owner.getSnapshot().snapshot?.revision !== revision) {
      clearTimeout(timer);
      stop();
      drop();
    }
  });
}

export async function timelineEdit(plan: Plan | { readonly problem: string }, origin: { readonly baseRevision?: number | undefined; readonly dir?: string | undefined } = {}): Promise<boolean> {
  if (previewing) return (previewPlan(plan), false);
  const snapshot = state.snapshot;
  const owner = currentEditorStore();
  if (!owner || !snapshot || !online() || !state.biliupCanEdit) return false;
  const work = editingFor(owner);
  // Planned in another project (it was switched mid-gesture): never send it to this one.
  if (origin.dir !== undefined && origin.dir !== snapshot.dir) return false;
  if (origin.baseRevision !== undefined && origin.baseRevision !== snapshot.revision) {
    set({ notice: { text: 'The project changed. Try the action again.' } });
    return false;
  }
  if ("problem" in plan) {
    set({ notice: { text: plan.problem } });
    return false;
  }
  if (!plan.ops.length) return false;
  if (work.adding) {
    set({ notice: { text: "The last change is still being saved. Try that again in a moment." } });
    return false;
  }
  // Checked with the server's timeline rules first, so a refusal reads as a sentence, not jargon.
  const draft = draftOf(projectOf(snapshot), plan.ops);
  if ("problem" in draft && origin.baseRevision === undefined) {
    set({ notice: { text: draft.problem } });
    return false;
  }
  const baseRevision = origin.baseRevision ?? snapshot.revision;
  playback.endHistory();
  work.adding = true;
  try {
    const error = await change(() => call("project.edit", { ops: plan.ops, summary: plan.summary, baseRevision }));
    return !error && currentEditorStore() === owner && state.snapshot?.dir === snapshot.dir;
  } finally {
    work.adding = false;
  }
}

/** One discrete command, one saved edit. Its result must not replace a newer selection. */
export async function timelineCommand(plan: Plan | { readonly problem: string }, after?: () => void): Promise<boolean> {
  const owner = currentEditorStore(), snapshot = state.snapshot, selected = getPlayback();
  if (!owner || !snapshot) return false;
  const changed = await itemEdit(() => plan, { dir: snapshot.dir, baseRevision: snapshot.revision });
  if (!changed || currentEditorStore() !== owner || state.snapshot?.dir !== snapshot.dir) return false;
  const now = getPlayback();
  if (now.selected === selected.selected && now.cut === selected.cut && now.gainKey === selected.gainKey
    && now.also.length === selected.also.length && now.also.every((id, index) => id === selected.also[index])) after?.();
  return true;
}

// Inspector and canvas edits run one after another: a colour click right after a size change
// waits for it instead of being refused as "still being saved", and each is planned against the
// project as it is when its turn comes (update_item replaces nested objects whole).
const settled = () => new Promise<void>((resolve) => setTimeout(resolve, 25));

/** Saves and history commands share ordering before they reach the project session. */
function enqueueEditorEdit<T>(operation: () => Promise<T>, cancelled: T): Promise<T> {
  const owner = currentEditorStore(), dir = state.snapshot?.dir;
  if (!owner || !dir || !online() || !state.biliupCanEdit) return Promise.resolve(cancelled);
  const work = editingFor(owner);
  work.queued++;
  let started = false;
  const run = async () => {
    try {
      while (work.adding && currentEditorStore() === owner) await settled();
      if (currentEditorStore() !== owner || state.snapshot?.dir !== dir || !online() || !state.biliupCanEdit) return cancelled;
      started = true; work.queued--;
      return await operation();
    } finally {
      if (!started) { started = true; work.queued--; }
    }
  };
  const next = work.chain.then(run, run);
  work.chain = next.catch(() => undefined);
  return next;
}

/** Undo/redo choose their target when their turn comes; restoring a named version keeps its base. */
export function queuedProjectChange(
  run: () => Promise<Result<ProjectSnapshot>> | undefined,
  { inline = false, origin }: { inline?: boolean; origin?: { dir: string; baseRevision: number } } = {},
): Promise<CallError | undefined> {
  const owner = currentEditorStore(), dir = state.snapshot?.dir;
  const cancelled: CallError = { _tag: 'EditRefused', message: editorText('The project changed. Try the action again.') };
  return enqueueEditorEdit<CallError | undefined>(async () => {
    if (origin && (state.snapshot?.dir !== origin.dir || state.snapshot.revision !== origin.baseRevision)) {
      if (!inline) set({ notice: { text: cancelled.message } });
      return cancelled;
    }
    const work = editingFor(owner!);
    work.adding = true;
    try {
      const request = run();
      if (!request) return;
      const error = await change(() => request, { inline });
      return currentEditorStore() === owner && state.snapshot?.dir === dir ? error : cancelled;
    } finally { work.adding = false; }
  }, cancelled);
}

/**
 * An item edit from the Inspector or the preview (D53): `makePlan` runs against the newest
 * project when it's this edit's turn. `baseRevision` is what the user saw when the gesture began
 * (a typing burst, a drag), so the server refuses it if the agent changed the item since (D6).
 */
export function itemEdit(makePlan: (project: ProjectModel) => Plan | { readonly problem: string }, origin: { readonly baseRevision?: number | undefined; readonly dir?: string | undefined } = {}): Promise<boolean> {
  const owner = currentEditorStore(), dir = state.snapshot?.dir;
  if (!owner || !dir || !online() || !state.biliupCanEdit) return Promise.resolve(false);
  if (previewing) {
    const snapshot = state.snapshot;
    if (snapshot && (origin.dir === undefined || origin.dir === snapshot.dir)) previewPlan(makePlan(projectOf(snapshot)));
    return Promise.resolve(false);
  }
  return enqueueEditorEdit(async () => {
    const snapshot = state.snapshot!;
    if (origin.dir !== undefined && origin.dir !== snapshot.dir) return false;
    return timelineEdit(makePlan(projectOf(snapshot)), { ...origin, dir });
  }, false);
}

/**
 * Shader effects registered in this project and definitions stored on its clips, by id.
 */
export function shaderDefs(project: ProjectModel | undefined): Array<ShaderDef & { readonly from: string }> {
  const out = new Map<string, ShaderDef & { readonly from: string }>();
  for(const def of state.projectShaders??[])out.set(def.id,{...def,from:'This project'});
  for (const t of project?.tracks ?? [])
    for (const it of t.items)
      for (const e of it.effects ?? []) {
        if (e.module !== SHADER) continue;
        const use = shaderUseOf(e);
        if (use && !out.has(use.def.id)) out.set(use.def.id, { ...use.def, from: "This project" });
      }
  return [...out.values()];
}
export const shaderDefOf = (project: ProjectModel, id: string) => shaderDefs(project).find((d) => d.id === id);
