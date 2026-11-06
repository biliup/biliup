import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Inspector for a selected cut's transition (D73).
import { transitionInfo, transitionLimits, transitionName, TRANSITIONS } from "@/app/lib/editor/core/visual/transitions";
import { AtSign, Blend, X } from "lucide-react";

import { cutFit, findItem, planSetTransition } from "@/app/ui/editor/timeline/edit";
import { transitionProblemText } from '@/app/ui/editor/timeline/edit/transitions';
import { itemLabel, type ProjectModel } from "@/app/lib/editor/state/project";
import { actions } from "@/app/lib/editor/state/store";
import { Group } from "@/app/ui/editor/inspector/fields";
import { NumInput } from "@/app/ui/editor/inspector/controls";
import { useProjectEdit } from "@/app/ui/editor/workspace/projectEdit";
import { editorGestureOrigin, type GestureOrigin } from "@/app/ui/editor/workspace/pointerGesture";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";

export function TransitionInspector({ project, placed, online }: { project: ProjectModel; placed: NonNullable<ReturnType<typeof findItem>>; online: boolean }) {
  useEditorLanguage();
  const edit = useProjectEdit();
  const fps = project.canvas.fps;
  const { item, index, track } = placed;
  const prev = track.items[index - 1]!;
  const tr = item.transitionIn;
  const locked = !online || track.locked === true || item.locked === true || prev.locked === true;
  const limits = tr ? transitionLimits(project, item.id, tr.module) : undefined;
  const range = limits && !("problem" in limits) ? limits : undefined;
  const set = (next: { module: string; duration?: number } | null, origin = editorGestureOrigin()) => void edit.commit(p => planSetTransition(p, item.id, next), origin);
  const duration = (frames: number, origin?: GestureOrigin) => tr && set({module: tr.module, duration: Math.round(frames)}, origin);
  const previewDuration = (frames: number) => { if (tr) edit.preview(project, planSetTransition(project, item.id, {module: tr.module, duration: Math.round(frames)})); };
  const secs = (frames: number) => Number((frames / fps).toFixed(2));
  return (
    <div data-testid="inspector-transition" data-item={item.id}>
      <div className="insp-title">
        <Blend />
        <span className="ellipsis grow">{tr ? editorText(transitionName(tr.module)) : editorText("Cut")}</span>
        {/* D264 (#77): the transition as a chip in the Agent tab's composer. */}
        {tr && (
          <Tooltip label={editorText("Add this transition to the agent chat")}>
            <button className="small-btn insp-chat" data-testid="insp-transition-add-to-chat" onClick={() => actions.addTransitionToChat(item.id)}>
              <AtSign />
              {editorText("Add to chat")}</button>
          </Tooltip>
        )}
      </div>
      <div className="insp-sub">
        {editorText("Between ")}{itemLabel(project, prev)} {editorText(" and ")}{itemLabel(project, item)} {editorText(" on ")}{track.name}
      </div>
      <Group label={editorText("Transition")}>
        <select className="field input" data-testid="insp-transition-type" value={tr?.module ?? ""} disabled={locked} onChange={(e) => set(e.target.value ? { module: e.target.value } : null)}>
          <option value="">{editorText("None (cut)")}</option>
          {TRANSITIONS.map((t) => {
            // F9 (D153): a transition that doesn't fit this cut is listed, but can't be picked.
            const fit = t.id === tr?.module ? undefined : cutFit(project, item.id, t.id);
            return (
              <option key={t.id} value={t.id} disabled={!!fit && !fit.fits} data-fit={fit ? (fit.fits ? "yes" : "no") : undefined}>
                {fit && !fit.fits ? `${editorText(t.name)} (${fit.short})` : editorText(t.name)}
              </option>
            );
          })}
          {tr && !transitionInfo(tr.module) && <option value={tr.module}>{transitionName(tr.module)} {editorText(" (draws as a crossfade)")}</option>}
        </select>
      </Group>
      {tr && (
        <Group label={editorText("Duration")}>
          <div className="field-row">
            <NumInput
              label={editorText("s")}
              testId="insp-transition-seconds"
              value={tr.duration / fps}
              step={1 / fps}
              min={range ? range.min / fps : 1 / fps}
              max={range ? range.max / fps : undefined}
              disabled={locked || !range}
              onPreview={v => previewDuration(v * fps)}
              onCancel={edit.cancel}
              onCommit={(v, origin) => v !== undefined && duration(v * fps, origin)}
            />
            <NumInput label={editorText("frames")} testId="insp-transition-frames" value={tr.duration} step={1} min={range?.min ?? 1} max={range?.max} disabled={locked || !range} onPreview={previewDuration} onCancel={edit.cancel} onCommit={(v, origin) => v !== undefined && duration(v, origin)} />
          </div>
          <div className="hint-line muted small" data-testid="insp-transition-range">
            {range ? editorText("{0}–{1} s ({2}–{3} frames) fit at this cut", { 0: secs(range.min), 1: secs(range.max), 2: range.min, 3: range.max }) : limits && "problem" in limits ? transitionProblemText(limits) : ""}
          </div>
        </Group>
      )}
      {tr && (
        <div className="group">
          <button className="btn" data-testid="insp-transition-remove" disabled={locked} onClick={() => set(null)}>
            <X />
            {editorText("Remove transition")}</button>
        </div>
      )}
    </div>
  );
}
