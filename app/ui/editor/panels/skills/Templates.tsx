import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Templates in the Skills tab (issue #111): Biliup's built-ins read-only (Use in chat, Duplicate) and
// the user's own (edit, delete), saved through biliup's shared resources. Use in chat fills
// the template from the open project and puts the brief in the composer, like the / menu does.
import type { TemplateInfo } from "@/app/lib/editor/contracts";
import { TEMPLATE_SYNTAX, templateId } from "@/app/lib/editor/core/templates/index";
import { ChevronLeft } from "lucide-react";

import { actions } from "@/app/lib/editor/state/store";
import { EditorFrame } from "@/app/ui/editor/panels/brand/parts";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import {useEditorAction} from '../../controls/useEditorAction';
import {isTextComposition} from '@/app/lib/editor/keyboard';

export interface TemplateDraft {
  readonly name: string;
  readonly description: string;
  readonly body: string;
}

/** A row in the Templates section. */
export function TemplateRow({ t, onOpen }: { t: TemplateInfo; onOpen: (t: TemplateInfo) => void }) {
  useEditorLanguage();
  return (
    <button className="brand-row" data-testid={`template-${t.id}`} onClick={() => onOpen(t)}>
      <span className="brand-row-top">
        <span className="brand-row-name ellipsis">{t.builtin ? editorText(t.name) : t.name}</span>
        {!t.builtin && <span className="brand-tag">{editorText("Yours")}</span>}
      </span>
      {t.description && <span className="brand-desc">{t.builtin ? editorText(t.description) : t.description}</span>}
    </button>
  );
}

/** One of the user's templates, new or existing. */
export function TemplateEditor({ id, draft, templates, online, onDraft, onClose }: { id: string | undefined; draft: TemplateDraft; templates: ReadonlyArray<TemplateInfo>; online: boolean; onDraft: (d: TemplateDraft) => void; onClose: () => void }) {
  useEditorLanguage();
  const {busy,error,editable,run}=useEditorAction();
  const set = (patch: Partial<TemplateDraft>) => onDraft({ ...draft, ...patch });
  const save = () => run(async()=>{
    if (!draft.name.trim()) return;
    const tid = id ?? templateId(draft.name, templates.map((t) => t.id));
    const ok = await actions.templatesSave({ id: tid, name: draft.name.trim(), description: draft.description.trim(), body: draft.body });
    return ok;
  },{onSuccess:ok=>{if(ok)onClose()}});
  const remove = () => run(async()=>{
    if (!id) return;
    const ok = await actions.templatesDelete(id);
    return ok;
  },{onSuccess:ok=>{if(ok)onClose()}});
  return (
    <EditorFrame title={draft.name.trim() || editorText("New template")} testId="template-editor" onClose={onClose} onSave={() => void save()} onDelete={id ? remove : undefined} saveDisabled={!online || !draft.name.trim()} busy={busy} readOnly={!editable} problem={error}>
      <div className="lib-section">
        <span className="lib-label">{editorText("Name")}</span>
        <input className="field input" data-testid="template-name" autoFocus={!id} maxLength={80} value={draft.name} onChange={(e) => set({ name: e.target.value })} />
        <span className="lib-label">{editorText("Description")}</span>
        <input className="field input" data-testid="template-description" maxLength={300} value={draft.description} onChange={(e) => set({ description: e.target.value })} />
        <span className="lib-label">{editorText("Brief")}</span>
        <div className="template-syntax" data-testid="template-syntax">
          {TEMPLATE_SYNTAX}
        </div>
        <textarea className="gen-prompt brand-text brand-mono" data-testid="template-body" spellCheck={false} value={draft.body} onChange={(e) => set({ body: e.target.value })} />
      </div>
    </EditorFrame>
  );
}

/** A template to read: Use in chat, Duplicate (a built-in is read-only), Edit for the user's own. */
export function TemplateView({ t, online, hasProject, onDuplicate, onEdit, onClose }: { t: TemplateInfo | undefined; online: boolean; hasProject: boolean; onDuplicate: (t: TemplateInfo) => void; onEdit: (t: TemplateInfo) => void; onClose: () => void }) {
  useEditorLanguage();
  const {busy,error,editable,run}=useEditorAction();
  const use = () => run(async()=>{
    if (!t || busy) return;
    return actions.useTemplate(t.id);
  });
  return (
    <div
      className="brand-editor"
      data-testid="template-view"
      onKeyDown={(e) => {
        if (e.key !== "Escape"||isTextComposition(e.nativeEvent)) return;
        e.stopPropagation();
        if(busy)return;
        onClose();
      }}
    >
      <div className="brand-editor-head">
        <Tooltip label={editorText("Back")}>
          <button className="icon-btn" data-testid="brand-back" disabled={busy} onClick={onClose}>
            <ChevronLeft />
          </button>
        </Tooltip>
        <span className="ellipsis">{t ? (t.builtin ? editorText(t.name) : t.name) : editorText("Template")}</span>
        <span className="brand-tag">{t?.builtin ? editorText("Template") : editorText("Yours")}</span>
      </div>
      <div className="brand-body">
        <div className="lib-section">
          {t?.description && <div className="brand-desc">{t.builtin ? editorText(t.description) : t.description}</div>}
          <pre className="brand-skill-read selectable" data-testid="template-view-body">
            {t?.body ?? ""}
          </pre>
        </div>
      </div>
      <div className="brand-foot">
        {error&&<p className="inline-error" role="alert">{error}</p>}
        <button className="btn ghost" data-testid="template-duplicate" disabled={!editable||busy||!t} onClick={() => t && onDuplicate(t)}>
          {editorText("Duplicate")}</button>
        {t && !t.builtin && (
          <button className="btn ghost" data-testid="template-edit" disabled={!editable||busy} onClick={() => onEdit(t)}>
            {editorText("Edit")}</button>
        )}
        <span className="brand-foot-gap" />
        <button className="btn ghost" data-testid="brand-cancel" disabled={busy} onClick={onClose}>
          {editorText("Close")}</button>
        <button className="btn primary" data-testid="template-use" disabled={!t || !hasProject || !online || !editable || busy} title={hasProject ? undefined : editorText("Open a project to use the agent")} onClick={() => void use()}>
          {editorText(busy ? "Loading…" : "Use in chat")}</button>
      </div>
    </div>
  );
}
