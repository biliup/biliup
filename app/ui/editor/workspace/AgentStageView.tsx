import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { editorContains, editorDialogOpen, editorEvents } from "@/app/lib/editor/environment";
import { isTextComposition } from '@/app/lib/editor/keyboard';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// #75 (D255): the agent centre stage on a new, empty project: the Agent tab (its suggestions, the
// thread and the composer) in a large card over the dimmed, blurred editor. The backdrop takes no
// pointer events: a click on the editor lands where it was aimed and sends the stage to the editor,
// as do Esc and "Go to editor". Media dropped on the card imports, as in the Media panel.
import { ArrowRight, Upload } from "lucide-react";
import React, { useEffect, useRef, useState } from "react";

import { AgentPanel } from "@/app/ui/editor/panels/Agent";
import { ImportReport } from "@/app/ui/editor/panels/library/ImportReport";
import { getState, useApp, type Project } from "@/app/lib/editor/state/store";
import { importDroppedFiles } from '@/app/lib/editor/state/store/server';
import type { StageDismissal } from "@/app/ui/editor/workspace/agentStage";

export function AgentStage({ project, online, onDismiss }: { project: Project; online: boolean; onDismiss: (how: StageDismissal) => void }) {
  useEditorLanguage();
  const { biliupCanEdit } = useApp();
  const writable = online && biliupCanEdit === true;
  const card = useRef<HTMLDivElement>(null);
  const [dropping, setDropping] = useState(false);
  useEffect(() => {
    const active = document.activeElement;
    if (editorDialogOpen() || (active && active !== document.body && (!editorContains(active) || active.matches('input, textarea, [contenteditable=true]')))) return;
    card.current?.querySelector<HTMLTextAreaElement>('textarea:not(:disabled)')?.focus({preventScroll:true});
  }, []);
  useEffect(() => {
    const dialog = () => {
      const s = getState();
      // D266: and any other dialog on screen (Import from link…), wherever the focus is.
      return s.exportOpen || s.settingsOpen || s.shortcutsOpen || editorDialogOpen();
    };
    // The composer's pickers take Esc first (they prevent its default).
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || e.defaultPrevented || isTextComposition(e) || dialog()) return;
      e.preventDefault();
      onDismiss("esc");
    };
    const onDown = (e: PointerEvent) => {
      const target = e.target as Element | null;
      if (!target || !editorContains(target) || card.current?.contains(target) || target.closest?.(".topbar, .modal-layer, .ctx-menu, [role=menu]") || dialog()) return;
      onDismiss("outside");
    };
    editorEvents.addEventListener("keydown", onKey);
    window.addEventListener("pointerdown", onDown, true);
    return () => {
      editorEvents.removeEventListener("keydown", onKey);
      window.removeEventListener("pointerdown", onDown, true);
    };
  }, [onDismiss]);

  const files = (e: React.DragEvent) => [...e.dataTransfer.types].includes("Files");
  const inComposer = (e: React.DragEvent) => e.target instanceof Element && !!e.target.closest('.composer');
  const onDrop = (e: React.DragEvent) => {
    if (!files(e) || inComposer(e)) return;
    e.preventDefault();
    e.stopPropagation();
    setDropping(false);
    if (writable) importDroppedFiles(e.dataTransfer);
  };

  return (
    <div className="agent-stage" data-testid="agent-stage">
      <div
        ref={card}
        className={`agent-stage-card ${dropping ? "dropping" : ""}`}
        data-testid="agent-stage-card"
        role="dialog"
        aria-label={editorText("Ask the agent")}
        onDragOverCapture={(e) => {
          if (!files(e) || inComposer(e)) { setDropping(false); return; }
          e.preventDefault();
          e.stopPropagation();
          e.dataTransfer.dropEffect = writable ? "copy" : "none";
          setDropping(writable);
        }}
        onDragLeave={(e) => {
          if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setDropping(false);
        }}
        onDropCapture={onDrop}
      >
        <div className="agent-stage-head">
          <span className="agent-stage-kicker">{editorText("New project")}</span>
          <span className="grow" />
          <button className="link-btn" data-testid="agent-stage-dismiss" title={editorText("Go to the editor (Esc)")} onClick={() => onDismiss("link")}>
            {editorText("Go to editor")}<ArrowRight />
          </button>
        </div>
        <AgentPanel project={project} online={online} />
        {/* D266: a drop that imported nothing says why here, not only in Media under the backdrop. */}
        <ImportReport testId="agent-stage-import-report" className="agent-stage-report" />
        {dropping && (
          <div className="agent-stage-drop" data-testid="agent-stage-drop">
            <Upload />
            {editorText("Import into the project")}</div>
        )}
      </div>
    </div>
  );
}
