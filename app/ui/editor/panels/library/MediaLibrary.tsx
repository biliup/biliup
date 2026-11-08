import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { ImportJob } from "@/app/lib/editor/contracts";
import { RefreshCw, Plus, Film, Upload, LayoutGrid, List, Scissors } from "lucide-react";
import { usedMediaIds, mediaUses } from "@/app/lib/editor/core/timeline/mediaPlan";
import React, { useState } from "react";
import { type Project, useApp, activeJobs, type MediaItem, actions, getState, projectOf } from "@/app/lib/editor/state/store";
import { useMediaView, sortMedia, setMediaSort, type MediaSort, MEDIA_SORTS, setMediaMode } from "@/app/ui/editor/panels/mediaView";
import { generatedMedia, GeneratedSection } from "@/app/ui/editor/panels/Generate";
import { RecentSection } from "@/app/ui/editor/panels/RecentMedia";
import { Empty, MediaTile } from "@/app/ui/editor/panels/library/MediaTile";
import { openMenu } from "@/app/ui/editor/timeline/ContextMenu";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import { ImportReport } from "@/app/ui/editor/panels/library/ImportReport";
import {useWorkbench} from '../../workspace/WorkbenchShell';
import {SourcePicker} from '../../workspace/SourcePicker';
import { chooseFiles, importDroppedFiles } from '@/app/lib/editor/state/store/server';
import {RecordingSelections} from './RecordingSelections';
import {RemoveMediaDialog} from './RemoveMediaDialog';

export function MediaLibrary({ project, onEdit }: { project: Project; onEdit?: () => void }) {
  useEditorLanguage();
  const app = useApp();
  const {project:session}=useWorkbench();
  const [importOpen,setImportOpen]=useState(false);
  const [selections,setSelections]=useState(false);
  const { connection, uploading } = app;
  const running = activeJobs(app);
  const importing = running.length > 0 || uploading !== undefined;
  const [dragging, setDragging] = useState(false);
  const online = connection === "connected";
  const editable = online && app.biliupCanEdit;
  // Generated media (D99) are listed under Generated, with their prompts.
  const generated = new Set(generatedMedia(project).map((m) => m.id));
  const view = useMediaView();
  const media = sortMedia(
    project.media.filter((m) => !generated.has(m.id)),
    view.sort,
  );
  // F26 (D168): which media the timeline uses; F8 (D167): the selected tile (⌫ removes it) and the confirm.
  const used = usedMediaIds(project);
  const [selected, setSelected] = useState<string | undefined>();
  const [confirm, setConfirm] = useState<{ readonly id: string; readonly path: string; readonly dir: string }>();
  const remove = (m: MediaItem) => {
    const snapshot = getState().snapshot;
    if (!editable || !snapshot || snapshot.dir !== app.snapshot?.dir) return;
    const current = projectOf(snapshot);
    if (!current.media.some(media => media.id === m.id && media.path === m.path)) return;
    if (mediaUses(current, m.id).length === 0) void actions.removeMedia(m.id, { dir: snapshot.dir, baseRevision: snapshot.revision });
    else setConfirm({ id: m.id, path: m.path, dir: snapshot.dir });
  };
  const onDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setDragging(false);
    if (editable) importDroppedFiles(e.dataTransfer);
  };
  if(selections&&session)return <RecordingSelections key={session.id} session={session} onClose={()=>setSelections(false)}/>;
  return (
    <aside
      className={`library ${dragging ? "dragging" : ""}`}
      data-testid="library"
      onDragOver={(e) => {
        if (!editable || ![...e.dataTransfer.types].includes("Files")) return;
        e.preventDefault();
        e.dataTransfer.dropEffect = "copy";
        setDragging(true);
      }}
      onDragLeave={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setDragging(false);
      }}
      onDrop={onDrop}
    >
      <div className="lib-head">
        {editorText("Media")}
        {session&&project.media.some(media=>app.biliupSources?.[media.id]?.kind==='recording')&&<button className="icon-btn" title={editorText('Recording selections')} aria-label={editorText('Recording selections')} onClick={()=>setSelections(true)}><Scissors/></button>}
        <button
          className="icon-btn"
          data-testid="import-plus"
          disabled={!editable}
          title={editorText("Import media")}
          onClick={(e) => {
            if(session){const r=e.currentTarget.getBoundingClientRect();openMenu(r.left,r.bottom+4,[
              {id:'import-files',label:editorText('Import files…'),run:()=>void actions.importMedia()},
              {id:'import-folder',label:editorText('Choose folder'),run:()=>chooseFiles(true)},
              {id:'import-recordings',label:editorText('Add recordings'),run:()=>setImportOpen(true)},
            ],editorText('Import media'));return;}
            void actions.importMedia();
          }}
        >
          {importing ? <RefreshCw className="spin" /> : <Plus />}
        </button>
      </div>
      {importOpen&&session&&<SourcePicker project={session} onClose={()=>setImportOpen(false)}/>}
      {uploading && (
        <div className="import-progress" data-testid="upload-progress">
          <RefreshCw className="spin" />
          <span>
            {editorText("Uploading ")}{uploading.name}
            {uploading.total > 1 ? editorText(" ({0} of {1})", { 0: uploading.done + 1, 1: uploading.total }) : ""}…
          </span>
        </div>
      )}
      {running.map((job) => (
        <div key={job.id} className="import-progress" data-testid="import-progress">
          <RefreshCw className="spin" />
          <span>{progressText(job)}</span>
          <div className="bar">
            <span style={{ width: `${Math.round(progressFraction(job) * 100)}%` }} />
          </div>
        </div>
      ))}
      {app.analysisError && (
        <div className="import-report" data-testid="analysis-error">
          <div className="import-line error">
            <span>{app.analysisError}</span>
          </div>
          <button className="link-btn" onClick={actions.dismissAnalysisError}>
            {editorText("Dismiss")}</button>
        </div>
      )}
      <ImportReport />
      {media.length === 0 && generated.size === 0 ? (
        <Empty icon={<Film />} title={editorText("No media yet")} testId="media-empty">
          <button className="btn" data-testid="import" disabled={!editable} onClick={() => void actions.importMedia()}>
            <Upload />
            {editorText("Import…")}</button>
        </Empty>
      ) : media.length > 0 ? (
        <div className="lib-section">
          <div className="lib-label media-bar">
            <span>{editorText("In this project")}</span>
            <span className="media-bar-tools">
              <select className="media-sort" data-testid="media-sort" value={view.sort} title={editorText("Sort by")} aria-label={editorText("Sort by")} onChange={(e) => setMediaSort(e.target.value as MediaSort)}>
                {MEDIA_SORTS.map((o) => (
                  <option key={o.id} value={o.id}>
                    {editorText(o.label)}
                  </option>
                ))}
              </select>
              <Tooltip label={editorText("Grid")}>
                <button className={`icon-btn${view.mode === "grid" ? " active" : ""}`} data-testid="media-view-grid" aria-pressed={view.mode === "grid"} onClick={() => setMediaMode("grid")}>
                  <LayoutGrid />
                </button>
              </Tooltip>
              <Tooltip label={editorText("List")}>
                <button className={`icon-btn${view.mode === "list" ? " active" : ""}`} data-testid="media-view-list" aria-pressed={view.mode === "list"} onClick={() => setMediaMode("list")}>
                  <List />
                </button>
              </Tooltip>
            </span>
          </div>
          <div className={view.mode === "list" ? "media-list" : "grid2"} data-testid="media-grid" data-view={view.mode} data-sort={view.sort}>
            {media.map((m) => (
              <MediaTile key={m.id} item={m} fps={project.canvas.fps} online={online} used={used.has(m.id)} selected={selected === m.id} onSelect={() => setSelected(m.id)} onRemove={() => remove(m)} />
            ))}
          </div>
        </div>
      ) : null}
      <GeneratedSection project={project} {...(onEdit ? { onEdit } : {})} mode={view.mode} used={used} selected={selected} onSelect={setSelected} onRemove={remove} />
      <RecentSection key={app.snapshot?.dir} project={project} />
      {dragging && <div className="drop-hint">{editorText("Drop to import")}</div>}
      {confirm && (
        <RemoveMediaDialog key={`${confirm.dir}:${confirm.id}:${confirm.path}`} mediaId={confirm.id} path={confirm.path} dir={confirm.dir} onClose={() => setConfirm(undefined)} />
      )}
    </aside>
  );
}

/** "Importing 37 of 200…" and friends. */
function progressText(job: ImportJob) {
  switch (job.phase) {
    case "queued":
      return `Waiting to import ${job.total} file${job.total === 1 ? "" : "s"}…`;
    case "probing":
      return `Importing ${Math.min(job.probed + 1, job.total)} of ${job.total}…`;
    case "adding":
      return `Adding ${job.total} file${job.total === 1 ? "" : "s"}…`;
    default:
      return `Making thumbnails, ${job.postersDone} of ${job.postersTotal}…`;
  }
}
const progressFraction = (job: ImportJob) =>
  job.phase === "posters" ? 0.8 + 0.2 * (job.postersTotal ? job.postersDone / job.postersTotal : 1) : job.phase === "probing" ? 0.75 * (job.probed / Math.max(1, job.total)) : job.phase === "adding" ? 0.78 : 0;
