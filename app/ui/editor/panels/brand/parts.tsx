import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { BrandScope } from "@/app/lib/editor/contracts";
import { ChevronLeft, Trash2 } from "lucide-react";
import type React from "react";
import { useState } from "react";
import { SCOPE_CHOICE } from "@/app/ui/editor/panels/brand/kit";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import {GlobalDialog} from '../../controls/GlobalDialog';
import {useApp} from '@/app/lib/editor/state/store';
import {isTextComposition} from '@/app/lib/editor/keyboard';

export function Swatches({ hexes }: { hexes: ReadonlyArray<string> }) {
  useEditorLanguage();
  return (
    <span className="brand-swatches">
      {hexes.slice(0, 12).map((hex, i) => (
        <span key={i} className="brand-swatch" style={{ background: hex }} title={hex} />
      ))}
    </span>
  );
}

export function ScopeChoice({ value, onChange, hasProject, testId, disabled=false }: { value: BrandScope; onChange: (scope: BrandScope) => void; hasProject: boolean; testId: string; disabled?:boolean }) {
  useEditorLanguage();
  return (
    // A segmented choice, not chips (D263).
    <div className="seg brand-scope" role="radiogroup" aria-label={editorText("Where it's kept")}>
      {(["global", "project"] as const).map((s) => (
        <button key={s} className={value === s ? "active" : ""} role="radio" aria-checked={value === s} data-testid={`${testId}-${s}`} disabled={disabled||s === "project" && !hasProject} onClick={() => onChange(s)}>
          {editorText(SCOPE_CHOICE[s])}
        </button>
      ))}
    </div>
  );
}

/** The editor's frame: back, the body, Save, Cancel and Delete; Esc drops the draft. */
export function EditorFrame({ title, testId, children, onClose, onSave, onDelete, deleteScope='global', saveDisabled, busy=false, readOnly=false, problem }: { title: string; testId: string; children: React.ReactNode; onClose: () => void; onSave: () => void; onDelete?: (() => Promise<unknown>) | undefined; deleteScope?:BrandScope; saveDisabled: boolean; busy?:boolean; readOnly?:boolean; problem?: string | undefined }) {
  useEditorLanguage();
  const {brandError}=useApp();
  const [confirm, setConfirm] = useState(false);
  return (
    <div
      className="brand-editor"
      data-testid={testId}
      onKeyDown={(e) => {
        if (e.key !== "Escape"||isTextComposition(e.nativeEvent)) return;
        e.preventDefault();e.stopPropagation();
        if(busy)return;
        if (confirm) setConfirm(false);
        else onClose();
      }}
    >
      <div className="brand-editor-head">
        <Tooltip label={editorText("Back")}>
          <button className="icon-btn" data-testid="brand-back" disabled={busy} onClick={onClose}>
            <ChevronLeft />
          </button>
        </Tooltip>
        <span className="ellipsis">{title}</span>
      </div>
      <div className="brand-body"><fieldset className="brand-fields" disabled={busy||readOnly}>{children}</fieldset></div>
      <div className="brand-foot">
        {problem && <div className="brand-problem small" role="alert" data-testid="brand-problem">{editorText(problem)}</div>}
        {onDelete && (
          <button className="btn ghost" data-testid="brand-delete" disabled={busy||readOnly} onClick={() => setConfirm(true)}>
            <Trash2 />
            {editorText("Delete")}</button>
        )}
        <span className="brand-foot-gap" />
        <button className="btn ghost" data-testid="brand-cancel" disabled={busy} onClick={onClose}>
          {editorText("Cancel")}</button>
        <button className="btn primary" data-testid="brand-save" disabled={saveDisabled||busy||readOnly} onClick={onSave}>
          {editorText("Save")}</button>
      </div>
      {confirm && onDelete && (
        <GlobalDialog label={editorText('Delete {0}?',{0:title})} busy={busy} onClose={()=>setConfirm(false)}>
          <div className="modal confirm-modal" data-testid="brand-delete-modal">
            <div className="confirm-title">{editorText('Delete {0}?',{0:title})}</div>
            <div className="confirm-text">{editorText(deleteScope==='project'?'It will be removed from this project.':'It goes from every project that uses it.')}</div>
            {(problem||brandError)&&<p className="inline-error" role="alert">{editorText(problem||brandError!)}</p>}
            <div className="confirm-actions">
              <button className="btn ghost" data-testid="brand-delete-cancel" disabled={busy} onClick={() => setConfirm(false)}>
                {editorText("Cancel")}</button>
              <button
                className="btn danger"
                data-testid="brand-delete-confirm"
                disabled={busy||readOnly}
                onClick={() => void onDelete()}
              >
                {editorText("Delete")}</button>
            </div>
          </div>
        </GlobalDialog>
      )}
    </div>
  );
}
