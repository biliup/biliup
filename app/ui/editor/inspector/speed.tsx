import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Inspector's speed (G9, D125) and names (T7, G4): speed presets, a custom speed and keep
// pitch for clips and audio items; the item's own name and its media's display name.
import { SPEED_PRESETS, SPEED_RANGE, speedLabel } from "@/app/lib/editor/core/timeline/arrange";
import type { Item } from "@/app/lib/editor/core/timeline/types";

import type { ProjectModel } from "@/app/lib/editor/state/project";
import { useApp } from "@/app/lib/editor/state/store";
import { Group } from "@/app/ui/editor/inspector/fields";
import { NumInput } from "@/app/ui/editor/inspector/controls";
import { planSpeed, planRename, readName, type NameTarget } from "@/app/ui/editor/timeline/edit";
import { useProjectEdit } from "@/app/ui/editor/workspace/projectEdit";
import { editorGestureOrigin, type GestureOrigin } from "@/app/ui/editor/workspace/pointerGesture";
import { usePlayback } from "@/app/ui/editor/preview/playback";
import { useTextDraft } from './text/draft';

/** Whether an item has a speed: a clip or audio item playing video or sound. */
export const hasSpeed = (project: ProjectModel, item: Item) => {
  if (item.type !== "clip" && item.type !== "audio") return false;
  const media = project.media.find((m) => m.id === item.media);
  return !!media && media.kind !== "image";
};

export function SpeedEditor({ project, item, locked }: { project: ProjectModel; item: Item; locked: boolean }) {
  useEditorLanguage();
  const edit = useProjectEdit();
  const linkOverlays = usePlayback(state => state.linkOverlays);
  const speed = item.speed ?? 1;
  const set = (value: number, keepPitch?: boolean, origin: GestureOrigin | undefined = editorGestureOrigin()) => edit.commit(p => planSpeed(p, item.id, value, keepPitch, linkOverlays), origin);
  return (
    <div data-testid="speed-editor">
      <div className="group">
        <div className="speed-row" role="group" aria-label={editorText("Speed")}>
          {SPEED_PRESETS.map((s) => (
            <button key={s} className={`small-btn ${Math.abs(s - speed) < 1e-6 ? "active" : ""}`} data-testid={`insp-speed-${s}`} aria-pressed={Math.abs(s - speed) < 1e-6} disabled={locked} onClick={() => void set(s)}>
              {speedLabel(s)}
            </button>
          ))}
        </div>
        <div className="field-row">
          <NumInput label="×" ariaLabel={editorText("Speed")} testId="insp-speed-custom" value={Math.round(speed * 1000) / 1000} step={0.05} min={SPEED_RANGE.min} max={SPEED_RANGE.max} disabled={locked}
            onPreview={value => edit.preview(project, planSpeed(project, item.id, value, undefined, linkOverlays))}
            onCancel={edit.cancel}
            onCommit={(value, origin) => value !== undefined && void set(value, undefined, origin)} />
        </div>
        <label className="check-row">
          <input type="checkbox" data-testid="insp-keep-pitch" checked={item.keepPitch !== false} disabled={locked} onChange={(e) => void set(speed, e.target.checked)} />
          {editorText("Keep pitch")}</label>
      </div>
    </div>
  );
}

/** A name field: committed on Enter or blur, Esc puts the saved one back. */
function NameField({ project, target, value, placeholder, testId, disabled, label }: { project: ProjectModel; target: NameTarget; value: string; placeholder?: string; testId: string; disabled: boolean; label: string }) {
  useEditorLanguage();
  const field = useTextDraft({project, value, disabled, multiline: false, saveOnPause: false, normalize: name => name.trim(), read: p => readName(p, target), plan: (p, name) => planRename(p, target, name)});
  return (
    <label className="field num-field">
      <span className="k">{label}</span>
      <input
        className="v"
        data-testid={testId}
        type="text"
        spellCheck={false}
        placeholder={placeholder}
        disabled={disabled}
        {...field.input}
      />
    </label>
  );
}

export function NamesEditor({ project, item, locked }: { project: ProjectModel; item: Item; locked: boolean }) {
  useEditorLanguage();
  const app = useApp();
  const editable = app.connection === 'connected' && !!app.biliupCanEdit;
  const media = item.media ? project.media.find((m) => m.id === item.media) : undefined;
  return (
    <Group label={editorText("Name")}>
      {item.type !== "captions" && <NameField key={`item:${item.id}`} project={project} target={{kind: 'item', id: item.id}} label={editorText("Item")} testId="insp-name" value={item.name ?? ""} placeholder={media?.name ?? ""} disabled={locked || !editable} />}
      {media && <NameField key={`media:${media.id}`} project={project} target={{kind: 'media', id: media.id}} label={editorText("Media")} testId="insp-media-name" value={media.name} disabled={!editable} />}
    </Group>
  );
}
