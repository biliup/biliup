import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Brand tab (D186, D232): Make a kit from a reference first (it calls the make-brand-kit skill
// in the composer: the agent asks a few questions, takes a link or dropped logos and builds the
// kit), the open project's kit, and the kits (logos, colours, fonts, voice, examples and a style
// guide the agent writes from a reference). Skills have their own tab since D232 (Skills.tsx).
// Kits live in the user's library (every project picks from it; D198) or are pinned to this
// project; an editor works on a draft and saves it explicitly (Esc or Cancel drop it). D198: logos show as pictures; logos and font files come in from a file, a folder or an
// https address; a font with its file is loaded everywhere text is drawn.
import type { BrandKit, BrandScope } from "@/app/lib/editor/contracts";
import { Plus, Sparkles, X } from "lucide-react";
import { useContext, useEffect, useState } from "react";

import { actions, brandFileUrl, getState, useApp } from "@/app/lib/editor/state/store";
import { BUILTIN_IDS } from "@/app/lib/editor/core/skills/skills";
import { type View, draftOfKit, freeId, emptyKit, SCOPE_LABEL, LOGO_BACKDROP } from "@/app/ui/editor/panels/brand/kit";
import { KitEditor } from "@/app/ui/editor/panels/brand/KitEditor";
import { ThisProject } from "@/app/ui/editor/panels/brand/ThisProject";
import { Swatches, ScopeChoice } from "@/app/ui/editor/panels/brand/parts";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import {GlobalDialog} from '../controls/GlobalDialog';
import {useEditorAction} from '../controls/useEditorAction';
import {EditorStoreContext} from '@/app/lib/editor/state/store/core';
import {isTextComposition} from '@/app/lib/editor/keyboard';
export { freeId } from "@/app/ui/editor/panels/brand/kit";

export function BrandLibrary() {
  useEditorLanguage();
  const app = useApp();
  const owner=useContext(EditorStoreContext),{busy,error,editable,run}=useEditorAction();
  const brand = app.brand;
  const online = app.connection === "connected";
  const hasProject = app.snapshot !== null;
  const [view, setView] = useState<View>({ kind: "list" });
  const [newKit, setNewKit] = useState<{ name: string; scope: BrandScope } | undefined>();
  useEffect(()=>{setView({kind:'list'});setNewKit(undefined)},[owner]);

  const openKit = (k: BrandKit) => setView({ kind: "kit", id: k.id, scope: k.scope, draft: draftOfKit(k) });
  // The shared resource catalog includes the built-in kit maker.
  const kitSkill = brand?.skills.some((s) => s.id === BUILTIN_IDS.brandKit);

  const createKit = () => run(async()=>{
    if (!newKit || !brand) return;
    const name = newKit.name.trim();
    if (!name) return;
    const id = freeId(name, brand.kits.map((k) => k.id), "kit");
    if (!(await actions.brandSaveKit(emptyKit(id, name, newKit.scope)))) return;
    return {id,scope:newKit.scope};
  },{onSuccess:result=>{
    if(!result)return;
    setNewKit(undefined);
    const made = getState().brand?.kits.find((k) => k.id === result.id && k.scope === result.scope);
    if (made) openKit(made);
  }});

  return (
    <aside className="library brand-panel" data-testid="brand-library">
      <div className="lib-head">{editorText("Brand")}</div>
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
        <div className="lib-section muted small" data-testid="brand-loading">
          {online ? editorText("Loading") : editorText("Not connected")}
        </div>
      ) : view.kind === "kit" ? (
        <KitEditor
          key={`${app.snapshot?.dir}:${view.scope}:${view.id}`}
          brand={brand}
          view={view}
          hasProject={hasProject}
          online={online}
          // An updater: a picked file lands in the draft as it is when the pick ends, if it's still open.
          onDraft={(f) => setView((v) => (v.kind === "kit" && v.id === view.id && v.scope === view.scope ? { ...v, draft: f(v.draft) } : v))}
          onClose={() => setView({ kind: "list" })}
        />
      ) : (
        <div className="brand-body">
          <div className="lib-section">
            <div className="brand-hero" data-testid="brand-hero">
              <span className="brand-hero-title">{editorText("Make a kit from a reference")}</span>
              <button
                className="btn primary"
                data-testid="brand-make-from-reference"
                disabled={!editable || !hasProject || !kitSkill}
                title={!hasProject ? editorText("Open a project to use the agent") : undefined}
                onClick={() => actions.composeSkill(BUILTIN_IDS.brandKit)}
              >
                <Sparkles />
                {editorText("Make from reference")}</button>
              {!hasProject && <span className="muted small">{editorText("Open a project to use the agent.")}</span>}
            </div>
          </div>
          <ThisProject brand={brand} hasProject={hasProject} online={online} />
          <div className="lib-section">
            <div className="brand-head">
              <span className="lib-label">{editorText("Kits")}</span>
              <button className="btn ghost brand-small-btn" data-testid="brand-new-kit" disabled={!editable||busy} onClick={() => setNewKit({ name: "", scope: "global" })}>
                <Plus />
                {editorText("New kit")}</button>
            </div>
            {brand.kits.length === 0 && <div className="muted small">{editorText("No kits yet")}</div>}
            {brand.kits.map((k) => (
              <button key={`${k.scope}:${k.id}`} className="brand-row" data-testid={`brand-kit-${k.id}`} onClick={() => openKit(k)}>
                <span className="brand-row-top">
                  <span className="brand-row-name ellipsis">{k.name}</span>
                  <span className="brand-tag">{SCOPE_LABEL[k.scope]}</span>
                </span>
                {k.colors.length > 0 && <Swatches hexes={k.colors.map((c) => c.hex)} />}
                {(() => {
                  const slot = (["light", "mark", "dark"] as const).find((x) => k.logos[x]);
                  const name = slot ? k.logos[slot]! : undefined;
                  const path = name ? brand.files[k.id]?.[name] : undefined;
                  const src = name && path ? brandFileUrl(k.id, name, path) : undefined;
                  return name ? (
                    <span className="brand-row-logo">
                      <span className="brand-logo-thumb" style={{ background: LOGO_BACKDROP[slot!] }}>
                        {src ? <img src={src} alt={name} draggable={false} /> : null}
                      </span>
                      <span className="brand-file ellipsis">{name}</span>
                    </span>
                  ) : null;
                })()}
                {k.style && <span className="brand-tag">{editorText("Style guide")}</span>}
              </button>
            ))}
          </div>
        </div>
      )}
      {newKit && (
        <GlobalDialog label={editorText('New kit')} busy={busy} onClose={()=>setNewKit(undefined)}>
          <div className="modal confirm-modal" data-testid="brand-new-kit-modal">
            <div className="confirm-title">{editorText("New kit")}</div>
            <input
              className="field input"
              data-testid="brand-new-kit-name"
              autoFocus
              placeholder={editorText("Name")}
              maxLength={80}
              disabled={busy||!editable}
              value={newKit.name}
              onChange={(e) => setNewKit({ ...newKit, name: e.target.value })}
              onKeyDown={(e) => {if(e.key==='Enter'&&!isTextComposition(e.nativeEvent)){e.preventDefault();void createKit()}}}
            />
            <ScopeChoice value={newKit.scope} hasProject={hasProject} disabled={busy||!editable} testId="brand-new-kit-scope" onChange={(scope) => setNewKit({ ...newKit, scope })} />
            {(error||app.brandError)&&<p className="inline-error" role="alert">{error||app.brandError}</p>}
            <div className="confirm-actions">
              <button className="btn ghost" data-testid="brand-new-kit-cancel" disabled={busy} onClick={() => setNewKit(undefined)}>
                {editorText("Cancel")}</button>
              <button className="btn primary" data-testid="brand-new-kit-create" disabled={!editable||busy||!newKit.name.trim()} onClick={() => void createKit()}>
                {editorText("Create")}</button>
            </div>
          </div>
        </GlobalDialog>
      )}
    </aside>
  );
}
