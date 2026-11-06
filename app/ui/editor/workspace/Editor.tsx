import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { focusEditor } from '@/app/lib/editor/environment';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { isLibraryPanel, type ProjectSnapshot } from "@/app/lib/editor/contracts";
import { Check, Focus, Minimize2, PanelBottom, PanelLeft, PanelRight, Redo2, RefreshCw, Settings as SettingsIcon, Share, TriangleAlert, Undo2 } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

import { useBeatsLoading } from "@/app/ui/editor/audio/beats";
import { useDuckLoading } from "@/app/ui/editor/audio/ducking";
import { MixerPanel } from "@/app/ui/editor/audio/Mixer";
import { ExportDialog } from "@/app/ui/editor/panels/ExportDialog";
import {activeExport} from '@/app/lib/editor/export/jobs';
import { AgentPanel } from "@/app/ui/editor/panels/Agent";
import { Library, type LibraryTab } from "@/app/ui/editor/panels/Library";
import { usePlayback } from "@/app/ui/editor/preview/playback";
import { Stage } from "@/app/ui/editor/preview/Preview";
import { actions, projectOf, useApp, type Project } from "@/app/lib/editor/state/store";
import { Timeline } from "@/app/ui/editor/timeline/Timeline";
import { ErrorToast } from "@/app/ui/editor/controls/ToastView";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import { ContextMenuHost } from "@/app/ui/editor/timeline/ContextMenu";
import { useEditorKeys } from "@/app/ui/editor/workspace/keys";
import { keysOf, type ShortcutId } from "@/app/ui/editor/workspace/keymap";
import { AgentStage } from "@/app/ui/editor/workspace/AgentStageView";
import { markDocked, stage, stageWanted, useStageDismissed, type StageDismissal } from "@/app/ui/editor/workspace/agentStage";
import { layoutStyle, panels, panelShown, panelsStyle, rememberSideTab, useLayout, usePanels, type Hideable, type Panels } from "@/app/ui/editor/workspace/workspaceLayout";
import { Rail } from "@/app/ui/editor/workspace/Rail";
import { Resizer } from "@/app/ui/editor/workspace/Resizer";
import { Side } from "@/app/ui/editor/workspace/Side";
import {WorkbenchToolbar,useWorkbench} from './WorkbenchShell';
import {RecordBar} from '../panels/capture/controls';

export function Editor({ snapshot }: { snapshot: ProjectSnapshot }) {
  useEditorLanguage();
  const project = projectOf(snapshot);
  const { connection, notice, exportOpen, panelRequest } = useApp();
  const online = connection === "connected";
  const {visible}=useWorkbench();
  const [tab, setTab] = useState<LibraryTab>("media");
  // Native actions and the assistant share one panel-opening path, including hidden panels.
  useEffect(() => {
    const panel = panelRequest?.panel;
    if (!panel) return;
    if (isLibraryPanel(panel)) { setTab(panel); panels.show("lib"); }
    else {
      rememberSideTab(snapshot.dir, panel);
      if (panel !== "agent" || !panels.get().focus) panels.show("side");
    }
  }, [panelRequest, snapshot.dir]);
  useEditorKeys(visible);
  useBeatsLoading(project, snapshot.dir);
  useDuckLoading(project, snapshot.dir);
  const sizes = useLayout();
  const preferences = usePanels();
  const shown = preferences;
  const { staged, dismiss: dismissStage } = useAgentStage(project, snapshot.dir, shown.focus);
  const preview=<Stage key={snapshot.dir} project={project} dir={snapshot.dir}/>;
  return (
    <div
      className="app"
      data-testid="editor"
      data-focus={shown.focus || undefined}
      data-hidden={(["lib", "side", "bottom"] as const).filter((p) => !panelShown(shown, p)).join(" ") || undefined}
      style={{ ...layoutStyle(sizes), ...panelsStyle(shown) }}
    >
      <TopBar snapshot={snapshot} online={online} shown={shown} captureVisible={tab==='capture'&&panelShown(shown,'lib')} />
      <ContextMenuHost />
      
      <div className="banners">
        {!online && (
          <div className="banner" data-testid="connection-banner" data-state={connection}>
            <RefreshCw className={connection === "reconnecting" ? "spin" : ""} />
            <span className="banner-text">
              {connection === "reconnecting"
                ? editorText("Reconnecting to the server… Changes are paused.")
                : connection === "failed"
                  ? editorText("The server is unavailable. Reconnect to continue editing.")
                  : editorText("Connecting to the server…")}
            </span>

          </div>
        )}
        {notice && (
          // #67 (D239): closes itself after a while; hovering it keeps it.
          // D262: a transport problem stays while the connection is still down (it goes on reconnect).
          <ErrorToast text={notice.text} token={notice} onClose={actions.dismissNotice} testId="editor-error" sticky={notice.transport === true && !online} />
        )}
      </div>
      {/* #76 (D256): hidden parts aren't rendered; focus mode is the preview and the agent beside it (D270). */}
      {panelShown(shown,'lib')&&<Rail tab={tab} onTab={setTab} visible={visible}/>}
      {panelShown(shown,'lib')&&<Library project={project} dir={snapshot.dir} tab={tab} onTab={setTab}/>}
      {preview}
      {panelShown(shown,'side')&&<Side snapshot={snapshot} project={project} online={online} staged={staged} onDock={() => dismissStage("link")} />}
      {shown.focus ? (
        <section className="focus-agent" data-testid="focus-agent">
          <AgentPanel project={project} online={online} />
        </section>
      ) : (
        panelShown(shown, "bottom") && <Bottom project={project} />
      )}
      {panelShown(shown, "lib") && <Resizer panel="lib" visible={visible} />}
      {panelShown(shown, "side") && <Resizer panel="side" visible={visible} />}
      {panelShown(shown, "bottom") && <Resizer panel="bottom" visible={visible} />}
      {/* #75 (D255): a new, empty project: the agent centre stage over the editor. */}
      {staged && <AgentStage project={project} online={online} onDismiss={dismissStage} />}
      {exportOpen && <ExportDialog key={snapshot.dir} project={project} dir={snapshot.dir} />}
    </div>
  );
}

/**
 * #75 (D255): whether the agent centre stage shows. It opens on an empty project whose agent hasn't
 * run (not in focus mode, where the agent is already beside the preview), and docks into the side
 * panel's Agent tab when that stops being so (a turn starts, media lands, the user goes to the
 * editor): the composer glides there in a view transition, and the side panel comes back if hidden.
 */
function useAgentStage(project: Project, dir: string, focus: boolean): { staged: boolean; dismiss: (how: StageDismissal) => void } {
  const { agent, biliupCanEdit } = useApp();
  const dismissed = useStageDismissed();
  const available = biliupCanEdit === true && agent?.configured === true;
  const staged = available && !focus && stageWanted(project, dir, agent, dismissed);
  // The chat goes on in the Agent tab (the side panel back if hidden), sliding in from the middle.
  const dock = useCallback(() => {
    // D266: docked is remembered like a dismissal, so undoing the import that docked it doesn't
    // bring the stage back over the editor (the Agent tab's empty state still can).
    stage.dismiss(dir);
    panels.show("side");
    markDocked(dir);
    rememberSideTab(dir, "agent");
  }, [dir]);
  // A click on the editor leaves the side panel's tab alone (they clicked something there); Esc and
  // "Go to editor" dock the chat into the Agent tab.
  const dismiss = useCallback(
    (how: StageDismissal) => {
      stage.dismiss(dir);
      if (how !== "outside") { dock(); focusEditor(); }
    },
    [dir, dock],
  );
  // The first turn or media landing (not a dismissal, not focus mode) docks it too.
  const was = useRef({ dir, staged });
  useEffect(() => {
    const before = was.current;
    was.current = { dir, staged };
    if (before.dir === dir && before.staged && !staged && available && !focus && !dismissed.includes(dir)) dock();
  }, [dir, staged, available, focus, dismissed, dock]);
  return { staged, dismiss };
}

/** The bottom panel (D32): the timeline or the mixer (D83). The transcript is a rail tab (U11, D177). */
function Bottom({ project }: { project: Project }) {
  useEditorLanguage();
  const bottom = usePlayback((s) => s.bottom);
  return bottom === "mixer" ? <MixerPanel project={project} /> : <Timeline project={project} />;
}

const TOGGLES: ReadonlyArray<{ panel: Hideable; icon: typeof PanelLeft; label: string; shortcut: ShortcutId }> = [
  { panel: "lib", icon: PanelLeft, get label() { return editorText("素材面板"); }, shortcut: "toggleLeft" },
  { panel: "bottom", icon: PanelBottom, get label() { return editorText("时间轴"); }, shortcut: "toggleTimeline" },
  { panel: "side", icon: PanelRight, get label() { return editorText("属性面板"); }, shortcut: "toggleRight" },
];

/** #76 (D256): the top bar's way to hide and bring back each part, and focus mode: always visible. */
function LayoutToggles({ shown }: { shown: Panels }) {
  useEditorLanguage();
  if (shown.focus)
    return (
      <Tooltip label={editorText("Leave focus mode")} shortcut="focusMode">
        <button className="focus-exit" data-testid="focus-exit" onClick={() => panels.focus(false)}>
          <Minimize2 />
          {editorText("Leave focus")}<kbd>{keysOf("focusMode")[0]}</kbd>
        </button>
      </Tooltip>
    );
  return (
    <div className="layout-toggles" role="group" aria-label={editorText("Panels")}>
      {TOGGLES.map(({ panel, icon: Icon, label, shortcut }) => {
        const on = panelShown(shown, panel);
        return (
          <Tooltip key={panel} label={editorText("{0} the {1}", { 0: editorText(on ? "Hide" : "Show"), 1: label })} shortcut={shortcut}>
            <button className="icon-btn" data-testid={`toggle-${panel}`} aria-pressed={on} onClick={() => panels.toggle(panel)}>
              <Icon />
            </button>
          </Tooltip>
        );
      })}
      <Tooltip label={editorText("Focus mode: only the preview and the agent")} shortcut="focusMode">
        <button className="icon-btn" data-testid="toggle-focus" aria-pressed={false} onClick={() => panels.focus(true)}>
          <Focus />
        </button>
      </Tooltip>
    </div>
  );
}

function TopBar({ snapshot, online, shown, captureVisible }: { snapshot: ProjectSnapshot; online: boolean; shown: Panels; captureVisible:boolean }) {
  useEditorLanguage();
  const { exportJobs, exportJobId, biliupSaving, biliupDirty, biliupRevision, biliupSaveError, biliupCanEdit } = useApp();
  const unsaved = !!biliupDirty;
  const job = activeExport(exportJobs,snapshot.dir,exportJobId);
  const running = !!job;
  return (
    <WorkbenchToolbar>
      {!captureVisible&&<RecordBar compact/>}
      <LayoutToggles shown={shown} />
      <div
        className={`saved ${unsaved ? "warn" : ""}`}
        data-testid="saved"
        title={biliupSaveError ?? (biliupSaving ? editorText("保存中") : unsaved ? editorText("未保存") : editorText("已保存到 biliup（版本 {0}）", { 0: biliupRevision ?? snapshot.revision }))}
      >
        {biliupSaving ? <RefreshCw className="spin" /> : unsaved ? <TriangleAlert /> : online ? <Check /> : <RefreshCw />}
        {biliupSaving ? editorText("保存中") : unsaved ? editorText("未保存") : online ? editorText("已保存") : editorText("离线")}
      </div>
      <Tooltip label={editorText("Undo")} shortcut="undo">
        <button className="icon-btn" data-testid="undo" aria-label={editorText("Undo")} disabled={!online || !biliupCanEdit || !snapshot.canUndo} onClick={() => void actions.undo()}>
          <Undo2 />
        </button>
      </Tooltip>
      <Tooltip label={editorText("Redo")} shortcut="redo">
        <button className="icon-btn" data-testid="redo" aria-label={editorText("Redo")} disabled={!online || !biliupCanEdit || !snapshot.canRedo} onClick={() => void actions.redo()}>
          <Redo2 />
        </button>
      </Tooltip>
      <Tooltip label={editorText("Export video")} shortcut="export">
      <button className="btn export-btn" data-testid="export-button" data-running={running} onClick={actions.openExport}>
        <Share />
        {running && job ? (job.phase === "rendering" ? editorText("Exporting {0}%", { 0: Math.round(job.progress * 100) }) : editorText("Exporting…")) : editorText("导出")}
      </button>
      </Tooltip>
      <Tooltip label={editorText("设置")} shortcut="settings">
        <button className="icon-btn" data-testid="open-settings" onClick={() => actions.openSettings()}>
          <SettingsIcon />
        </button>
      </Tooltip>
    </WorkbenchToolbar>
  );
}
