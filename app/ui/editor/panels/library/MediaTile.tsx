import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { AtSign, AudioLines, AudioWaveform, Image as ImageIcon, Film, Plus, RefreshCw, Sparkles, X } from "lucide-react";
import React, { useState, useRef, useEffect, type ReactNode } from "react";
import { timecode } from "@/app/lib/editor/state/format";
import { type MediaItem, useApp, actions, getState, activeAnalyses, thumbUrl, filmstripUrl, waveformUrl, mediaUrl, proxyOf, proxyUrl, projectOf } from "@/app/lib/editor/state/store";
import { planReplaceMarks } from "@/app/ui/editor/timeline/replace";
import { useProxiesOn } from "@/app/ui/editor/settings/proxies";
import { beginEditorDrag, endEditorDrag } from "@/app/ui/editor/timeline/dnd";
import { openMenu } from "@/app/ui/editor/timeline/ContextMenu";
import { RenameField } from "@/app/ui/editor/timeline/RenameField";
import { analysisText } from "@/app/lib/editor/state/analysisText";
import { actionFor, displayKey, keymap, keyOf } from "@/app/ui/editor/workspace/keymap";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import { getPlayback, playback } from "@/app/ui/editor/preview/playback";
import {fallbackProxy} from '@/app/ui/editor/preview/compositor/media';
import {MediaVideo} from '@/app/ui/editor/preview/MediaVideo';
import { usePoster, usePreviewVisible, retryMediaPreviews } from '@/app/ui/editor/preview/resources';
import { useWorkbench } from '../../workspace/WorkbenchShell';
import { ReplaceMediaDialog } from './ReplaceMediaDialog';
import type { EditorSession } from '@/app/lib/editor/session';
import { currentEditorStore } from '@/app/lib/editor/state/store/core';
import { isTextComposition } from '@/app/lib/editor/keyboard';
import { focusEditor } from '@/app/lib/editor/environment';
import { revealItem } from '@/app/lib/editor/state/store/edits';
import {captureEditorFocus} from '@/app/ui/editor/preview/focus';

/** "59.94", "30", "23.98" */
const fpsLabel = (fps: number) => String(Math.round(fps * 100) / 100);

/** "3840×2160, 59.94 fps" for video, "4096×3072" for images, "Audio" for sound. */
export function mediaMeta(m: MediaItem) {
  const size = m.probe?.width && m.probe?.height ? `${m.probe.width}×${m.probe.height}` : undefined;
  if (m.kind === "audio") return editorText("Audio");
  if (m.kind === "image") return size ?? editorText("Image");
  return [size, m.probe?.fps ? `${fpsLabel(m.probe.fps)} fps` : undefined].filter(Boolean).join(", ");
}

/** What a tile in Media › Generated adds (D240, #74): its prompt and model, Again and Edit prompt. */
export type GeneratedTileInfo = {
  readonly prompt: string;
  /** The generation model and its configured provider. */
  readonly by: string;
  /** Again works (the model's key is set and there is a prompt). */
  readonly canAgain: boolean;
  readonly againTitle: string;
  readonly again: () => void;
  readonly edit: () => void;
};

/** D240 (#73): the Analyze button's tooltip; it used to be an unexplained speedometer. */
export const ANALYZE_TITLE = "Analyse the sound: find its silences and loudness (used for silence cuts and auto-ducking)";

export function MediaTile({ item, fps, online, used, selected, onSelect, onRemove, generated }: { item: MediaItem; fps: number; online: boolean; used: boolean; selected: boolean; onSelect: () => void; onRemove: () => void; generated?: GeneratedTileInfo }) {
  useEditorLanguage();
  const app = useApp();
  const editable = online && app.biliupCanEdit;
  const { project: session } = useWorkbench();
  const [replacement, setReplacement] = useState<{ session: EditorSession; target: MediaItem }>();
  // G4: renamed in place (double-click the name, or Rename in its menu).
  const [renaming, setRenaming] = useState(false);
  const tileRef = useRef<HTMLDivElement>(null);
  const visible = usePreviewVisible(tileRef);
  // "Show in Media" (T7): scrolled into view and flashed.
  const focusSeq = app.mediaFocus?.id === item.id ? app.mediaFocus.seq : undefined;
  const [flash, setFlash] = useState(false);
  useEffect(() => {
    if (focusSeq === undefined) return;
    tileRef.current?.scrollIntoView({ block: "nearest" });
    const frame = requestAnimationFrame(() => setFlash(true));
    const t = setTimeout(() => setFlash(false), 1200);
    return () => { cancelAnimationFrame(frame); clearTimeout(t); };
  }, [focusSeq]);
  /** Issue #118: In to Out on the timeline plays this media from frame `from`; nothing after it moves. */
  const prepareReplaceMarks = (from: number) => {
    const now = getState().snapshot;
    const owner = currentEditorStore();
    if (!now || !owner || !projectOf(now).media.some(media => media.id === item.id && media.path === item.path)) return undefined;
    const { markIn, markOut, selected } = getPlayback();
    const plan = planReplaceMarks(projectOf(now), item.id, { markIn, markOut, selected }, from);
    return async () => {
      if (currentEditorStore() !== owner) return;
      const current = getState().snapshot;
      if (!current || current.dir !== now.dir) return;
      if (current.revision !== now.revision) {
        await actions.timelineEdit({ problem: 'The project changed. Try the action again.' }, { dir: now.dir });
        return;
      }
      const unchanged=captureEditorFocus();
      const applied = await actions.timelineEdit(plan, { baseRevision: now.revision, dir: now.dir });
      if (applied && unchanged() && 'piece' in plan) {
        playback.pause(); playback.select(plan.piece); playback.seek(markIn!); revealItem(plan.piece);
      }
    };
  };
  const menu = (e: React.MouseEvent) => {
    e.preventDefault();
    // The frame (canvas fps) skimmed on the thumbnail when the menu opened: where a replacement starts.
    const at = scrubTime!==undefined?Math.floor(scrubTime*fps):0;
    const replaceMarks = prepareReplaceMarks(at);
    openMenu(e.clientX, e.clientY, [
      // D264 (#77): the media (or a generated picture) as a chip in the Agent tab's composer.
      { id: "media-add-to-chat", get label() { return editorText("Add to chat"); }, run: () => void actions.addMediaToChat(item.id) },
      { id: "media-refresh-preview", get label() { return editorText("Refresh previews"); }, disabled: !online, run: () => {
        setBroken(undefined);
        retryMediaPreviews({poster:posterUrl,filmstrip:filmstripUrl(item.id,item.path),waveform:waveformUrl(item.id,item.path)});
      } },
      "-",
      ...(generated
        ? [
            { id: "generated-add", get label() { return editorText("Add at playhead"); }, disabled: !editable, run: () => void actions.addAtPlayhead(item.id) },
            { id: "generated-again", get label() { return editorText("Make again"); }, disabled: !editable || !generated.canAgain, run: generated.again },
            { id: "generated-edit", get label() { return editorText("Edit prompt"); }, disabled: !online, run: generated.edit },
          ]
        : [
            { id: "media-add", get label() { return editorText("Add to timeline"); }, disabled: !editable, run: () => void actions.addMedia(item.id) },
          ]),
      // Every source uses the same overwrite edit, including audio and generated media.
      { id: "media-replace-range", get label() { return editorText("Replace In to Out with this"); }, disabled: !editable || !replaceMarks, run: () => void replaceMarks?.() },
      ...(session ? [{ id: "media-replace-file", get label() { return editorText("Replace file"); }, disabled: !online || !app.biliupCanEdit || !!app.biliupDirty, run: () => setReplacement({ session, target: item }) }] : []),
      { id: "media-rename", get label() { return editorText("Rename"); }, disabled: !editable, run: () => setRenaming(true) },
      ...(proxy&&(proxy.phase==='queued'||proxy.phase==='running')?[{id:'proxy-cancel',get label(){return editorText('Cancel proxy')},disabled:!online||!app.biliupCanEdit,run:()=>void actions.cancelProxy(item.id)}]:[]),
      ...(proxy&&(proxy.phase==='failed'||proxy.phase==='cancelled')?[{id:'proxy-retry',get label(){return editorText('Retry proxy')},disabled:!online||!app.biliupCanEdit,run:()=>void actions.retryProxy(item.id)}]:[]),
      ...(failedRestore?[{id:'analysis-restore-retry',get label(){return editorText('Retry analyses')},disabled:!online||!app.biliupCanEdit,run:()=>void actions.retryAnalyses(item.id,failedRestore.id)}]:[]),
      "-",
      { id: "media-remove", get label() { return editorText("Remove from project"); }, shortcut: displayKey('⌫'), danger: true, disabled: !editable, run: onRemove },
    ], "Media");
  };
  // Analysis (D66) runs on demand: the Analyze button, or the agent's media_analyze.
  const analyses = activeAnalyses(app, item.id);
  const failedRestore=app.analysisJobs.find(job=>job.mediaId===item.id&&job.version===item.path&&job.analyzer==='restore'&&job.phase==='failed');
  const analysis = analyses.find(job => job.phase === 'running') ?? analyses[0];
  const hasSound = !generated && (item.kind === "audio" || (item.kind === "video" && item.probe?.hasAudio !== false));
  const [broken, setBroken] = useState<string>();
  const Icon = item.kind === "audio" ? AudioLines : item.kind === "image" ? ImageIcon : Film;
  const posterUrl = item.kind !== 'audio' ? thumbUrl(item.id, item.path) : undefined;
  const poster = usePoster(posterUrl, visible);
  const src = poster.value?.src;
  const seconds = item.probe?.frames ? item.probe.frames / fps : undefined;
  // D212: the preview proxy of heavy footage (made in the background after import).
  const proxy = proxyOf(app, item);
  const proxiesOn = useProxiesOn();
  // F26 (D168): hovering a video's thumbnail scrubs through it (one muted video, only while hovered).
  const [scrub, setScrub] = useState<number | undefined>(undefined);
  const canScrub = item.kind === "video" && !!seconds && !renaming;
  const scrubSource=proxiesOn&&proxy?.phase==='done'&&proxy.key?proxyUrl(item.id,proxy.key):mediaUrl(item.id,item.path);
  const scrubTime=scrub!==undefined&&seconds?Math.min(Math.max(0,seconds-0.05),Math.max(0,scrub*seconds)):undefined;
  const clearSkim=()=>{if(getPlayback().hoverMedia?.media===item.id)playback.hoverMedia(undefined)};
  // A tile that goes while skimmed (removed, another project) leaves the preview to the playhead.
  useEffect(() => {
    const owner = currentEditorStore();
    return () => { if (currentEditorStore() === owner && getPlayback().hoverMedia?.media === item.id) playback.hoverMedia(undefined); };
  }, [item.id, item.path]);
  return (
    <><div
      className={`tile ${flash ? "flash" : ""} ${selected ? "selected" : ""}${generated ? " gen-tile" : ""}`}
      data-testid={generated ? "generated-item" : "media-tile"}
      data-id={item.id}
      data-used={used ? "true" : undefined}
      data-selected={selected ? "true" : undefined}
      title={generated ? `${item.name}\n${generated.prompt || editorText("No prompt saved")}\n${generated.by}` : item.name}
      ref={tileRef}
      tabIndex={0}
      onClick={onSelect}
      onKeyDown={(e) => {
        if (isTextComposition(e.nativeEvent)) return;
        // Keyboard-focused controls keep native activation; Delete on one must not remove
        // an unrelated timeline selection. The focused tile itself retains media shortcuts.
        if(e.target instanceof Element&&e.target.closest('button')&&[' ','Enter','Backspace','Delete'].includes(e.key)){
          e.stopPropagation();
          if(e.key==='Backspace'||e.key==='Delete')e.preventDefault();
          return;
        }
        // D264: the add-to-chat key (⌘L) on the focused tile adds this media, not the timeline's selection.
        if (e.target === e.currentTarget) {
          const pressed = keyOf(e.nativeEvent);
          if (pressed !== undefined && actionFor(keymap.bindings(), pressed) === "addToChat") {
            e.preventDefault();
            e.stopPropagation();
            if (!e.repeat) actions.addMediaToChat(item.id);
            return;
          }
        }
        // ⌫ / Delete on the selected tile (not while its name is edited): Remove from project, never the timeline's selection.
        if (e.target !== e.currentTarget || (e.key !== "Backspace" && e.key !== "Delete")) return;
        e.preventDefault();
        e.stopPropagation();
        if (editable && !e.repeat && !e.metaKey && !e.ctrlKey && !e.altKey) onRemove();
      }}
      onKeyUp={e=>{if(e.target instanceof Element&&e.target.closest('button')&&(e.key===' '||e.key==='Enter'))e.stopPropagation()}}
      onContextMenu={(e) => {
        onSelect();
        menu(e);
      }}
      draggable={editable && !renaming}
      onDragStart={(e) => {
        if (!editable || !beginEditorDrag({kind:'media',id:item.id,path:item.path}, e.dataTransfer)) e.preventDefault();
        else {setScrub(undefined);clearSkim()}
      }}
      onDragEnd={endEditorDrag}
    >
      <div
        className="thumb"
        onPointerMove={(e) => {
          if ((e.target as Element).closest('button')) return;
          if (!canScrub) return;
          const r = e.currentTarget.getBoundingClientRect();
          const at = Math.min(1, Math.max(0, (e.clientX - r.left) / Math.max(1, r.width)));
          setScrub(at);
          // Issue #118: and the main preview shows that frame too (skimming), while no button is held.
          if (e.buttons === 0) playback.hoverMedia(item.id, Math.min(Math.max(0, seconds! - 0.05), Math.max(0, at * seconds!)));
        }}
        onPointerLeave={() => {
          setScrub(undefined);
          clearSkim();
        }}
        onPointerDown={clearSkim}
      >
        {src && broken !== src ? <img key={src} src={src} alt="" draggable={false} onError={() => setBroken(src)} data-testid="thumb" /> : <Icon />}
        {(poster.phase === 'failed' || !!src && broken === src) && <button className="icon-btn tile-preview-retry" aria-label={editorText('Retry preview')} title={editorText('Retry preview')} onPointerDown={event => event.stopPropagation()} onClick={event => {
          event.stopPropagation();setBroken(undefined);retryMediaPreviews({poster:posterUrl});
        }}><RefreshCw /></button>}
        {canScrub && scrub !== undefined && (
          <>
            <MediaVideo media={item} position={scrubTime} className="thumb-scrub" data-testid="thumb-scrub" src={scrubSource} onError={e=>fallbackProxy(e.currentTarget,scrubSource,mediaUrl(item.id,item.path))} muted playsInline preload="auto" />
            <span className="thumb-scrub-bar" style={{ left: `${scrub * 100}%` }} />
          </>
        )}
        {used && (
          <span className="tile-used" data-testid="media-used" title={editorText("On the timeline")}>
            {editorText("Used")}</span>
        )}
        {generated && (
          <span className="tile-gen-badge" data-testid="generated-badge" title={editorText("Generated: made in the Generate tab")} aria-label={editorText("Generated")} role="img">
            <Sparkles />
          </span>
        )}
        {proxy && !generated && (
          <span
            className={`tile-proxy ${proxy.phase}`}
            data-testid="media-proxy"
            data-phase={proxy.phase}
            title={proxy.phase==='failed' ? proxy.error : proxy.phase==='cancelled' ? editorText('Proxy cancelled') : proxy.phase === "done" ? (proxiesOn ? editorText("The preview plays a proxy of this file; export uses the original.") : editorText("Proxy made; the preview plays the original (Settings).")) : proxy.phase === "running" ? editorText("Making a proxy for the preview") : proxy.waiting ? editorText("Proxy waits for the export to finish") : editorText("Proxy queued")}
            style={proxy.phase === "running" ? { backgroundImage: `linear-gradient(90deg, rgb(255 255 255 / 22%) ${Math.round((proxy.progress ?? 0) * 100)}%, transparent 0)` } : undefined}
          >
            {proxy.phase==='failed' ? editorText('Proxy failed') : proxy.phase==='cancelled' ? editorText('Proxy cancelled') : proxy.phase === "running" ? editorText("Proxy {0}%", { 0: Math.round((proxy.progress ?? 0) * 100) }) : editorText("Proxy")}
          </span>
        )}
        {seconds !== undefined && !analysis && (
          <span className="dur" data-testid="duration">
            {timecode(Math.round(seconds * fps), fps).replace(/^00:/, "")}
          </span>
        )}
        <Tooltip label={generated ? editorText("Put it on the timeline at the playhead") : item.kind === "audio" ? editorText("Add to the audio track") : editorText("Add to the end of the main track")}>
          <button
            className="tile-add"
            data-testid={generated ? "generated-add" : "add-to-timeline"}
            disabled={!editable}
            onClick={(e) => {
              e.stopPropagation();
              focusEditor();
              void (generated ? actions.addAtPlayhead(item.id) : actions.addMedia(item.id));
            }}
          >
            <Plus />
          </button>
        </Tooltip>
        <Tooltip label={editorText("Add to chat")} shortcut="addToChat">
          <button
            className={`tile-chat${generated || (hasSound && !analysis) ? " second" : ""}`}
            data-testid="media-add-to-chat"
            onClick={(e) => {
              e.stopPropagation();
              focusEditor();
              actions.addMediaToChat(item.id);
            }}
          >
            <AtSign />
          </button>
        </Tooltip>
        {generated && (
          <Tooltip label={generated.againTitle}>
            <button
              className="tile-analyze"
              data-testid="generated-again"
              aria-label={editorText("Make again")}
              disabled={!editable || !generated.canAgain}
              onClick={(e) => {
                e.stopPropagation();
                focusEditor();
                generated.again();
              }}
            >
              <RefreshCw />
            </button>
          </Tooltip>
        )}
        {(hasSound || item.kind === 'video') && !analysis && (
          <Tooltip label={editorText(item.kind === 'video' ? 'Analyse footage' : ANALYZE_TITLE)}>
            <button
              className="tile-analyze"
              data-testid="analyze"
              aria-label={editorText(item.kind === 'video' ? 'Analyse footage' : "Analyse the sound")}
              disabled={!editable||app.analysisStarting?.includes(item.id)}
              onClick={(e) => {
                focusEditor();
                e.stopPropagation();
                void actions.analyzeMedia(item.id, item.kind === 'video' ? (hasSound ? ['video', 'loudness'] : ['video']) : undefined);
              }}
            >
              {item.kind === 'video' ? <Film /> : <AudioWaveform />}
            </button>
          </Tooltip>
        )}
        {analysis && (
          <div className="tile-analysis" data-testid="analysis-progress" data-phase={analysis.phase} data-analyzer={analysis.analyzer} title={analysisText(analysis)}>
            {analysis.analyzer === 'video' ? <Film /> : <AudioWaveform />}
            <div className="bar">
              <span style={{ width: `${Math.round((analysis.phase === "running" ? analysis.progress : 0) * 100)}%` }} />
            </div>
            <Tooltip label={editorText("Stop the analysis")}>
              <button className="tile-analysis-cancel" data-testid="analysis-cancel" disabled={!editable||analyses.some(job=>app.analysisCancelling?.includes(job.id))} onClick={(e) => {e.stopPropagation(); void Promise.all(analyses.map(job => actions.cancelAnalysis(job.id)));}}>
                <X />
              </button>
            </Tooltip>
          </div>
        )}
      </div>
      <div className="tile-name" data-testid="tile-name" title={item.name} onDoubleClick={() => editable && setRenaming(true)}>
        {renaming ? <RenameField testId="media-rename" value={item.name} onDone={(name,origin) => (name !== undefined && name.trim() && name !== item.name && void actions.renameMedia(item.id, name,origin), setRenaming(false))} /> : item.name}
      </div>
      <div className="tile-sub" data-testid="tile-meta">
        {generated ? <span className="gen-by">{generated.by}</span> : mediaMeta(item)}
      </div>
    </div>
    {replacement && replacement.session === session && <ReplaceMediaDialog session={replacement.session} target={replacement.target} onClose={() => setReplacement(undefined)} />}
    </>
  );
}

export function Empty({ icon, title, children, testId }: { icon: ReactNode; title: string; children?: ReactNode; testId?: string }) {
  useEditorLanguage();
  return (
    <div className="empty" data-testid={testId}>
      <div className="empty-icon">{icon}</div>
      <div className="empty-title">{title}</div>
      <div className="empty-sub">{children}</div>
    </div>
  );
}
