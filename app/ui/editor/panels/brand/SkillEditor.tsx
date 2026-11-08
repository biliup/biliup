import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { BrandState, BrandSkill } from "@/app/lib/editor/contracts";
import { ChevronLeft } from "lucide-react";
import { actions } from "@/app/lib/editor/state/store";
import { type View, freeId } from "@/app/ui/editor/panels/brand/kit";
import { EditorFrame, ScopeChoice } from "@/app/ui/editor/panels/brand/parts";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import {useEditorAction} from '../../controls/useEditorAction';

export function SkillEditor({ brand, view, hasProject, online, onDraft, onClose }: { brand: BrandState; view: Extract<View, { kind: "skill" }>; hasProject: boolean; online: boolean; onDraft: (d: Extract<View, { kind: "skill" }>["draft"]) => void; onClose: () => void }) {
  useEditorLanguage();
  const d = view.draft;
  const {busy,error,editable,run}=useEditorAction();
  const set = (patch: Partial<typeof d>) => onDraft({ ...d, ...patch });
  const save = () => run(async(current)=>{
    if (!d.name.trim()) return;
    const id = view.id ?? freeId(d.name, brand.skills.map((s) => s.id), "skill");
    let ok = await actions.brandSaveSkill({ id, name: d.name.trim(), scope: d.scope, description: d.description.trim(), body: d.body });
    if (ok && current() && view.id && d.scope !== view.scope) ok = await actions.brandDeleteSkill(view.id, view.scope);
    return ok;
  },{onSuccess:ok=>{if(ok)onClose()}});
  const remove = () => run(async()=>{
    if (!view.id) return;
    const ok = await actions.brandDeleteSkill(view.id, view.scope);
    return ok;
  },{onSuccess:ok=>{if(ok)onClose()}});
  return (
    <EditorFrame
      title={d.name.trim() || editorText("New skill")}
      testId="brand-skill-editor"
      onClose={onClose}
      onSave={() => void save()}
      onDelete={view.id ? remove : undefined}
      deleteScope={view.scope}
      saveDisabled={!online || !d.name.trim()}
      busy={busy} readOnly={!editable} problem={error}
    >
      <div className="lib-section">
        <span className="lib-label">{editorText("Name")}</span>
        <input className="field input" data-testid="brand-skill-name" autoFocus={!view.id} maxLength={80} value={d.name} onChange={(e) => set({ name: e.target.value })} />
        <span className="lib-label">{editorText("Where")}</span>
        <ScopeChoice value={d.scope} hasProject={hasProject} testId="brand-skill-scope" onChange={(scope) => set({ scope })} />
        <span className="lib-label">{editorText("Description")}</span>
        <input className="field input" data-testid="brand-skill-description" maxLength={300} value={d.description} onChange={(e) => set({ description: e.target.value })} />
        <span className="lib-label">{editorText("Instructions")}</span>
        <textarea className="gen-prompt brand-text brand-mono" data-testid="brand-skill-body" spellCheck={false} value={d.body} onChange={(e) => set({ body: e.target.value })} />
      </div>
    </EditorFrame>
  );
}

/** Built-in instructions can be used directly or copied into an editable user skill. */
export function SkillView({ skill, online, hasProject, onUse, onDuplicate, onClose }: { skill: BrandSkill | undefined; online: boolean; hasProject: boolean; onUse: (s: BrandSkill) => void; onDuplicate: (s: BrandSkill) => void; onClose: () => void }) {
  useEditorLanguage();
  return (
    <div
      className="brand-editor"
      data-testid="skill-view"
      onKeyDown={(e) => {
        if (e.key !== "Escape") return;
        e.stopPropagation();
        onClose();
      }}
    >
      <div className="brand-editor-head">
        <Tooltip label={editorText("Back")}>
          <button className="icon-btn" data-testid="brand-back" onClick={onClose}>
            <ChevronLeft />
          </button>
        </Tooltip>
        <span className="ellipsis">{skill ? (skill.builtin ? editorText(skill.name) : skill.name) : editorText("Skill")}</span>
        <span className="brand-tag">/{skill?.id ?? ""}</span>
      </div>
      <div className="brand-body">
        <div className="lib-section">
          {skill?.description && <div className="brand-desc">{editorText(skill.description)}</div>}
          <pre className="brand-skill-read selectable" data-testid="skill-view-body">
            {skill?.body ?? ""}
          </pre>
        </div>
      </div>
      <div className="brand-foot">
        <button className="btn ghost" data-testid="brand-duplicate" disabled={!online || !skill} onClick={() => skill && onDuplicate(skill)}>
          {editorText("Duplicate")}</button>
        <span className="brand-foot-gap" />
        <button className="btn ghost" data-testid="brand-cancel" onClick={onClose}>
          {editorText("Close")}</button>
        <button className="btn primary" data-testid="skill-use" disabled={!online || !skill || !hasProject} title={hasProject ? undefined : editorText("Open a project to use the agent")} onClick={() => skill && onUse(skill)}>
          {editorText("Use in chat")}</button>
      </div>
    </div>
  );
}
