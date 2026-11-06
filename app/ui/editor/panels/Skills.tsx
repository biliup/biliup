import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Built-in and user-authored editing skills share the editor's resource library.
import type { BrandSkill, TemplateInfo } from "@/app/lib/editor/contracts";
import { Plus, X } from "lucide-react";
import { useContext, useEffect, useState } from "react";

import { actions, useApp } from "@/app/lib/editor/state/store";
import { type View, SCOPE_LABEL } from "@/app/ui/editor/panels/brand/kit";
import { SkillEditor, SkillView } from "@/app/ui/editor/panels/brand/SkillEditor";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import { TemplateEditor, TemplateRow, TemplateView, type TemplateDraft } from "@/app/ui/editor/panels/skills/Templates";
import {EditorStoreContext} from '@/app/lib/editor/state/store/core';

// Issue #111: the Templates section's own views beside the skills'.
type TabView = View | { readonly kind: "template"; readonly id: string } | { readonly kind: "template-edit"; readonly id: string | undefined; readonly draft: TemplateDraft };

export function SkillsLibrary() {
  useEditorLanguage();
  const app = useApp();
  const owner=useContext(EditorStoreContext);
  const brand = app.brand;
  const online = app.connection === "connected";
  const editable=online&&!!app.biliupCanEdit;
  const hasProject = app.snapshot !== null;
  const [view, setView] = useState<TabView>({ kind: "list" });
  useEffect(()=>setView({kind:'list'}),[owner]);
  const templates = app.templates ?? [];
  // Reload the shared resource library when the tab opens.
  useEffect(() => {
    if (!online) return;
    void actions.brandGet();
    void actions.templatesGet();
  }, [online,owner]);
  const duplicateTemplate = (t: TemplateInfo) => setView({ kind: "template-edit", id: undefined, draft: { name: editorText("{0} copy", {0:t.builtin ? editorText(t.name) : t.name}), description: t.builtin ? editorText(t.description) : t.description, body: t.body } });

  const openSkill = (s: BrandSkill) =>
    s.builtin ? setView({ kind: "builtin", id: s.id }) : setView({ kind: "skill", id: s.id, scope: s.scope, draft: { name: s.name, scope: s.scope, description: s.description, body: s.body } });
  const newSkill = (t?: Pick<BrandSkill, "name" | "description" | "body">) =>
    setView({ kind: "skill", id: undefined, scope: "global", draft: { name: t ? editorText('{0} copy',{0:editorText(t.name)}) : "", scope: "global", description: t?.description ?? "", body: t?.body ?? "" } });
  const use = (s: BrandSkill) => actions.composeSkill(s.id);

  const yours = brand?.skills ?? [];
  const row = (s: BrandSkill) => (
    <button key={`${s.scope}:${s.id}`} className="brand-row" data-testid={`skill-${s.id}`} onClick={() => openSkill(s)}>
      <span className="brand-row-top">
        <span className="brand-row-name ellipsis">{s.builtin ? editorText(s.name) : s.name}</span>
        <span className="brand-tag">{s.builtin ? editorText("Built in") : editorText(SCOPE_LABEL[s.scope])}</span>
        <span className="skill-call">/{s.id}</span>
      </span>
      {s.description && <span className="brand-desc">{s.builtin ? editorText(s.description) : s.description}</span>}
    </button>
  );

  return (
    <aside className="library brand-panel skills-panel" data-testid="skills-library">
      <div className="lib-head">{editorText("Skills")}</div>
      {app.brandError && (
        <div className="inline-error small brand-error" data-testid="brand-error">
          <span>{app.brandError}</span>
          <Tooltip label={editorText("Dismiss")}>
            <button className="icon-btn" data-testid="brand-error-dismiss" onClick={actions.dismissBrandError}>
              <X />
            </button>
          </Tooltip>
        </div>
      )}
      {!brand ? (
        <div className="lib-section muted small" data-testid="skills-loading">
          {online ? editorText("Loading") : editorText("Not connected")}
        </div>
      ) : view.kind === "builtin" ? (
        <SkillView skill={brand.skills.find((s) => s.id === view.id)} online={editable} hasProject={hasProject} onUse={use} onDuplicate={(s) => newSkill(s)} onClose={() => setView({ kind: "list" })} />
      ) : view.kind === "template" ? (
        <TemplateView
          t={templates.find((t) => t.id === view.id)}
          online={online}
          hasProject={hasProject}
          onDuplicate={duplicateTemplate}
          onEdit={(t) => setView({ kind: "template-edit", id: t.id, draft: { name: t.name, description: t.description, body: t.body } })}
          onClose={() => setView({ kind: "list" })}
        />
      ) : view.kind === "template-edit" ? (
        <TemplateEditor id={view.id} draft={view.draft} templates={templates} online={online} onDraft={(draft) => setView({ ...view, draft })} onClose={() => setView({ kind: "list" })} />
      ) : view.kind === "skill" ? (
        <SkillEditor brand={brand} view={view} hasProject={hasProject} online={online} onDraft={(draft) => setView({ ...view, draft })} onClose={() => setView({ kind: "list" })} />
      ) : (
        <div className="brand-body">
          <div className="lib-section">
            <div className="brand-head">
              <span className="lib-label" data-testid="skills-yours">{editorText("Skills")}</span>
              <button className="btn ghost brand-small-btn" data-testid="skills-new" disabled={!editable} onClick={() => newSkill()}>
                <Plus />
                {editorText("New skill")}</button>
            </div>
            {yours.length === 0 && <div className="muted small">{editorText("Create a skill to reuse your editing instructions")}</div>}
            {yours.map(row)}
          </div>
          <div className="lib-section" data-testid="templates-section">
            <div className="brand-head">
              <span className="lib-label">{editorText("Templates")}</span>
              <button className="btn ghost brand-small-btn" data-testid="templates-new" disabled={!editable} onClick={() => setView({ kind: "template-edit", id: undefined, draft: { name: "", description: "", body: "" } })}>
                <Plus />
                {editorText("New template")}</button>
            </div>
            {templates.map((t) => (
              <TemplateRow key={t.id} t={t} onOpen={(x) => setView({ kind: "template", id: x.id })} />
            ))}
          </div>
        </div>
      )}
    </aside>
  );
}
