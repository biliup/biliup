import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { SoundFolderInfo } from "@/app/lib/editor/contracts";
import { type LibraryKind, filterFolderSounds, libraryMediaOf, lengthLabel } from "@/app/lib/editor/core/audio/library";
import { Folder, FolderPlus, RefreshCw, X, Square, Play, Plus } from "lucide-react";
import { useState, useEffect, useRef, useContext } from "react";
import { type Project, useApp, actions, folderSoundUrl } from "@/app/lib/editor/state/store";
import { type Preview, Wave, TogetherButton } from "@/app/ui/editor/panels/audio/preview";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import { ImportReport } from '@/app/ui/editor/panels/library/ImportReport';
import {EditorStoreContext} from '@/app/lib/editor/state/store/core';

/** Render in batches; every matching sound remains reachable without a new query. */
const FOLDER_ROWS = 200;
const sizeLabel = (bytes: number) => (bytes >= 1024 * 1024 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1024))} KB`);
type Result = ReadonlyArray<SoundFolderInfo> | { readonly error: string } | undefined;

/** Browser-imported sound folders, shared by music and sound effects. */
export function FolderSounds({ kind, project, online, query, preview }: { kind: LibraryKind; project: Project; online: boolean; query: string; preview: Preview }) {
  useEditorLanguage();
  const app = useApp();
  const owner=useContext(EditorStoreContext);
  const [all, setAll] = useState<ReadonlyArray<SoundFolderInfo> | undefined>();
  const [error, setError] = useState<string | undefined>();
  const [busy, setBusy] = useState(false);
  const [updating, setUpdating] = useState<string>();
  const [visibleRows, setVisibleRows] = useState(FOLDER_ROWS);
  const currentRun = useRef(0);
  const busyRef = useRef(false);
  const apply = (r: Result) => {
    if (!r) return;
    if ("error" in r) return setError(r.error);
    setError(undefined);
    setAll(r);
  };
  const run = async (task: () => Promise<Result>, folder?: string) => {
    if (busyRef.current) return;
    busyRef.current = true;
    const request = ++currentRun.current;
    setBusy(true);
    setUpdating(folder);
    setError(undefined);
    try {
      const result = await task();
      if (currentRun.current === request) apply(result);
    } catch (error) {
      if (currentRun.current === request) setError(error instanceof Error ? error.message : String(error));
    } finally {
      if (currentRun.current === request) { busyRef.current = false; setBusy(false); setUpdating(undefined); }
    }
  };
  useEffect(() => {
    setAll(undefined);setBusy(false);setUpdating(undefined);setError(undefined);
    if (online) void run(() => actions.soundFolders());
    return () => { currentRun.current++; busyRef.current = false; };
  }, [online, owner]);
  useEffect(() => { setVisibleRows(FOLDER_ROWS); }, [query, all, owner]);
  const folders = all;
  const sounds = folders ? filterFolderSounds(folders, query) : [];

  return (
    <div className="lib-folders" data-testid="folder-sounds" data-busy={busy}>
        <div className="lib-actions">
          <button className="btn ghost" data-testid="folder-add" disabled={!online || busy || !app.biliupCanEdit} onClick={() => void run(() => actions.addSoundFolder())}>
            <FolderPlus /> {editorText(" Add a folder")}</button>
          <Tooltip label={editorText("Refresh")}>
            <button className="icon-btn" data-testid="folder-refresh" disabled={!online || busy} onClick={() => void run(() => actions.soundFolders())}>
              <RefreshCw className={busy ? "spin" : ""} />
            </button>
          </Tooltip>
        </div>
      {error && (
        <div className="lib-limit error" data-testid="folder-error" role="alert">
          {error}
        </div>
      )}
      {busy && app.uploading && <div className="muted small" role="status">{editorText('Importing {0} ({1}/{2})', {0:app.uploading.name,1:app.uploading.done+1,2:app.uploading.total})}</div>}
      <ImportReport testId="folder-import-report" />
      {folders && folders.length === 0 && (
        <div className="muted small" data-testid="folder-empty">
          {editorText("No sound files here yet.")}</div>
      )}
      {!!folders?.length && (
        <div className="lib-folder-list" data-testid="sound-folders">
          {folders.map((f) => (
            <div key={f.id} className="lib-folder" data-testid="sound-folder" data-id={f.id} title={f.name}>
              <Folder />
              <span className="lib-folder-name">{f.name}</span>
              <span className="lib-folder-count">{f.sounds.length}</span>
              <Tooltip label={editorText('Update this folder')}>
                <button className="icon-btn" aria-label={editorText('Update {0}',{0:f.name})} data-testid="sound-folder-update" disabled={!online || busy || !app.biliupCanEdit} onClick={() => void run(() => { preview.stop(); return actions.addSoundFolder(f); }, f.id)}>
                  <RefreshCw className={updating===f.id?'spin':''} />
                </button>
              </Tooltip>
              <Tooltip label={editorText("Remove this folder from the list (its files stay)")}>
                <button
                  className="icon-btn"
                  data-testid="sound-folder-remove"
                  disabled={!online || busy || !app.biliupCanEdit}
                 
                  onClick={() => void run(() => { preview.stop(); return actions.removeSoundFolder(f.id); })}
                >
                  <X />
                </button>
              </Tooltip>
            </div>
          ))}
        </div>
      )}
      <div data-testid="folder-list">
        {folders && folders.some((f) => f.sounds.length) && sounds.length === 0 && (
          <div className="muted small" data-testid="folder-none">
            {query.trim() ? editorText("Nothing matches \"{0}\".", { 0: query.trim() }) : editorText("No sound files here yet.")}
          </div>
        )}
        {sounds.slice(0, visibleRows).map((f) => {
          const key = `folder:${f.id}`;
          const url = folderSoundUrl(f.id);
          const wave=f.seconds!==undefined&&f.seconds>0&&f.seconds<=120&&f.bytes<=20*1024*1024;
          const playing = preview.playing === key && preview.mode === "alone";
          const inProject = !!libraryMediaOf(project, key);
          const peaks = preview.peaksFor(key,url);
          const artist = f.folderName;
          return (
            <div key={f.id} className="music-row" data-testid="folder-row" data-id={f.id} title={f.rel}>
              <button className={`icon-btn${playing ? " playing" : ""}`} disabled={!online} data-testid="folder-play" data-playing={playing} title={playing ? editorText("Stop") : editorText("Listen")} onClick={() => preview.toggle(key, url, wave)}>
                {playing ? preview.loading?<RefreshCw className="spin"/>:<Square /> : <Play />}
              </button>
              <span className="music-text">
                <span className="music-title">{f.name}</span>
                <span className="music-meta">{[artist, f.ext, ...(f.seconds===undefined?[]:[lengthLabel(f.seconds)]), sizeLabel(f.bytes), ...(inProject ? [editorText("in the project")] : [])].filter(Boolean).join(", ")}</span>
                {peaks && <Wave peaks={peaks} progress={preview.playing === key ? preview.progress : 0} />}
              </span>
              {kind === "sfx" && <TogetherButton preview={preview} soundKey={key} url={url} wave={wave} />}
              <button
                className="icon-btn"
                data-testid="folder-add-sound"
                disabled={!online || !app.biliupCanEdit || !!app.libraryAdding}
                title={editorText("Copy into the project and add to an audio track at the playhead")}
                onClick={(e) => {
                  e.currentTarget.blur();
                  void actions.addLibrarySound(key, kind === "music");
                }}
              >
                {app.libraryAdding === key ? <RefreshCw className="spin" /> : <Plus />}
              </button>
            </div>
          );
        })}
        {sounds.length > visibleRows && (
          <button className="btn ghost" onClick={() => setVisibleRows(count => count + FOLDER_ROWS)}>
            {editorText('More sounds ({0})', {0:sounds.length-visibleRows})}
          </button>
        )}
      </div>
    </div>
  );
}
