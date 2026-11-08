import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type LibraryHit, type LibraryKind, hitKey, libraryMediaOf, lengthLabel, LICENSE_NAMES, filterSfx, type SfxSound } from "@/app/lib/editor/core/audio/library";
import { Square, Play, RefreshCw, Plus } from "lucide-react";
import { type Project, libraryPreviewUrl, actions, useApp, sfxUrl } from "@/app/lib/editor/state/store";
import { type Preview, Wave, TogetherButton } from "@/app/ui/editor/panels/audio/preview";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";

export function HitRow({ hit, kind, project, online, preview, adding }: { hit: LibraryHit; kind: LibraryKind; project: Project; online: boolean; preview: Preview; adding: boolean }) {
  useEditorLanguage();
  const app=useApp();
  const key = hitKey(hit);
  const playing = preview.playing === key && preview.mode === "alone";
  const inProject = !!libraryMediaOf(project, key);
  const url=libraryPreviewUrl(hit.source,hit.id),peaks=preview.peaksFor(key,url);
  const wave=hit.seconds!==undefined&&hit.seconds>0&&hit.seconds<=120;
  const meta = [hit.creator, ...(hit.seconds ? [lengthLabel(hit.seconds)] : []), ...(hit.provider && hit.provider !== hit.source ? [hit.provider] : []), ...(inProject ? [editorText("in the project")] : [])];
  return (
    <div className="music-row lib-row" data-testid="library-row" data-id={key} title={hit.attribution}>
      <button className={`icon-btn${playing ? " playing" : ""}`} disabled={!online} data-testid="library-play" data-playing={playing} title={playing ? editorText("Stop") : editorText("Listen")} onClick={() => preview.toggle(key, url, wave)}>
        {playing ? preview.loading?<RefreshCw className="spin"/>:<Square /> : <Play />}
      </button>
      <span className="music-text">
        <span className="music-title">{editorText(hit.title)}</span>
        <span className="music-meta">{meta.join(", ")}</span>
        {peaks && <Wave peaks={peaks} progress={preview.playing === key ? preview.progress : 0} />}
      </span>
      {kind === "sfx" && <TogetherButton preview={preview} soundKey={key} url={url} wave={wave} />}
      <span className={`lib-license ${hit.license}`} data-testid="library-license" title={hit.license === "by" ? editorText("Needs credit: Copy credits lists it") : editorText("Public domain, no credit needed")}>
        {LICENSE_NAMES[hit.license]}
      </span>
      <button
        className="icon-btn"
        data-testid="library-add"
        disabled={!online || !app.biliupCanEdit || !!app.libraryAdding}
        title={kind === "music" ? editorText("Add to an audio track at the playhead (as long as the video, with fades)") : editorText("Add to an audio track at the playhead")}
        onClick={(e) => {
          e.currentTarget.blur();
          void actions.addLibrarySound(key, kind === "music");
        }}
      >
        {adding ? <RefreshCw className="spin" /> : <Plus />}
      </button>
    </div>
  );
}

/** The bundled sound-effects pack (D134, D250): CC0 sounds matching the search and the kind select, listen and +. */
export function IncludedSfx({ project, online, query, category, preview }: { project: Project; online: boolean; query: string; category: string | undefined; preview: Preview }) {
  useEditorLanguage();
  const sounds = filterSfx({ query, category });
  return (
    <div data-testid="sfx-included">
      <div data-testid="sfx-list">
        {sounds.length === 0 && (
          <div className="muted small" data-testid="sfx-empty">
            {editorText("No sound matches. Try other words, or generate one.")}</div>
        )}
        {sounds.map((s) => (
          <SfxRow key={s.id} sound={s} project={project} online={online} preview={preview} />
        ))}
      </div>
    </div>
  );
}

function SfxRow({ sound: s, project, online, preview }: { sound: SfxSound; project: Project; online: boolean; preview: Preview }) {
  useEditorLanguage();
  const app = useApp();
  const key = `sfx:${s.id}`;
  const playing = preview.playing === key && preview.mode === "alone";
  const inProject = !!libraryMediaOf(project, key);
  // The pack's peaks come with it (make-sfx, D208): the waveform shows before it's played.
  const peaks = preview.peaksFor(key,sfxUrl(s.file)) ?? s.peaks.map((p) => p / 100);
  return (
    <div className="music-row" data-testid="sfx-row" data-id={s.id} title={s.provider === "uncut" ? editorText("{0}: made by Uncut, CC0", { 0: s.title }) : editorText("{0} by {1}, CC0: {2}", { 0: s.title, 1: s.author, 2: s.source })}>
      <button className={`icon-btn${playing ? " playing" : ""}`} disabled={!online} data-testid="sfx-play" data-playing={playing} title={playing ? editorText("Stop") : editorText("Listen")} onClick={() => preview.toggle(key, sfxUrl(s.file), false)}>
        {playing ? preview.loading?<RefreshCw className="spin"/>:<Square /> : <Play />}
      </button>
      <span className="music-text">
        <span className="music-title">{editorText(s.title)}</span>
        <span className="music-meta">{[s.author, lengthLabel(s.seconds), ...(inProject ? [editorText("in the project")] : [])].join(", ")}</span>
        {peaks && <Wave peaks={peaks} progress={preview.playing === key ? preview.progress : 0} />}
      </span>
      <TogetherButton preview={preview} soundKey={key} url={sfxUrl(s.file)} wave={false} />
      <Tooltip label={editorText("Add to an audio track at the playhead")}>
        <button
          className="icon-btn"
          data-testid="sfx-add"
          disabled={!online || !app.biliupCanEdit || !!app.libraryAdding}
         
          onClick={(e) => {
            e.currentTarget.blur();
            void actions.addLibrarySound(key, false);
          }}
        >
          {app.libraryAdding===key?<RefreshCw className="spin"/>:<Plus />}
        </button>
      </Tooltip>
    </div>
  );
}
