import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { EditWord } from "@/app/lib/editor/core/text/transcript";
import { Captions, ChevronDown, X, Settings } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Popover } from "@/app/ui/editor/inspector/controls";
import { type CaptionMode, CAPTION_DEFAULTS, CAPTION_MODES, CAPTION_MODE_NAMES, CAPTION_STYLES, captionStyleDef } from "@/app/lib/editor/core/text/captions";
import { TEXT_STYLES } from "@/app/lib/editor/core/text/looks";
import { analysisText } from "@/app/lib/editor/state/analysisText";
import { usePlayback, getPlayback } from "@/app/ui/editor/preview/playback";
import type { ProjectModel } from "@/app/lib/editor/state/project";
import { useApp, activeAnalyses, actions } from "@/app/lib/editor/state/store";
import { wordKey, useTranscripts, doneTranscripts, planCaptions, scopeOf, type CaptionOptions } from "@/app/ui/editor/panels/transcript";
import { makeCaptions } from "@/app/ui/editor/panels/transcriptView/cuts";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import {useDefaultCaptionStyle} from '../../inspector/captionStyle';
import {currentEditorStore} from '@/app/lib/editor/state/store/core';

/**
 * The Captions row (#59): what's there and one button; the choices (how they show, the look) open in
 * the shared Popover, which is drawn over everything and kept inside the window, so a narrow panel
 * can't clip it.
 */
export function CaptionsButton({ project, media, disabled, busy, transcribed }: { project: ProjectModel; media: ReadonlyArray<string>; disabled: boolean; busy:boolean; transcribed: boolean }) {
  useEditorLanguage();
  const [open, setOpen] = useState(false);
  const [mode, setMode] = useState<CaptionMode | undefined>();
  const [style, setStyle] = useState('');
  const anchor = useRef<HTMLButtonElement>(null);
  const mounted=useRef(true),submitting=useRef(false);
  useEffect(()=>{mounted.current=true;return()=>{mounted.current=false}},[]);
  const existing = project.tracks.some((t) => t.items.some((i) => i.type === "captions" && media.includes(i.source ?? "")));
  const transcripts=useTranscripts(state=>state.entries),source=useTranscripts(state=>state.source);
  const defaultStyle=useDefaultCaptionStyle();
  const options = useMemo<CaptionOptions>(() => ({
    ...(mode ? {mode} : {}),
    ...(style.startsWith('caption:') ? {captionStyle: style.slice(8)} : style ? {look: style} : {}),
  }), [mode, style]);
  const preview=useMemo(()=>open?planCaptions(project,doneTranscripts(transcripts),options,scopeOf(project,source)):undefined,[open,project,transcripts,source,options]);
  const fixesDropped=preview&&'fixesDropped' in preview?preview.fixesDropped:0;
  const close = useCallback(() => setOpen(false), []);
  const make=async()=>{
    if(disabled||busy||submitting.current||!preview||'problem' in preview)return;
    const owner=currentEditorStore();
    if(!owner)return;
    submitting.current=true;
    try{if(await makeCaptions(options,fixesDropped))if(mounted.current&&currentEditorStore()===owner)setOpen(false)}
    finally{submitting.current=false}
  };
  return (
    <div className="tx-tool tx-captions">
      <div className="tx-tool-text">
        <div className="tx-tool-title">{editorText("Captions")}</div>
        <div className="tx-tool-meta">{!transcribed ? editorText("Transcribe first") : existing ? editorText("On the timeline") : editorText("From the transcript")}</div>
      </div>
      <button
        ref={anchor}
        className={`btn ${open ? "active" : ""}`}
        data-testid="make-captions"
        aria-label={existing ? editorText("Captions") : editorText("Make captions")}
        aria-expanded={open && !disabled}
        disabled={disabled||busy}
        title={disabled ? editorText("Transcribe the speech first: captions are made from the transcript") : editorText("Captions from the transcript: they follow your cuts")}
        onClick={() => {
          if (!open) {
            const preset = captionStyleDef(defaultStyle);
            setStyle(existing ? '' : preset ? `caption:${preset.id}` : CAPTION_DEFAULTS.look);
            setMode(existing ? undefined : preset?.mode ?? CAPTION_DEFAULTS.mode);
          }
          setOpen(!open);
        }}
      >
        {existing ? editorText("Remake") : editorText("Make")}
        <ChevronDown />
      </button>
      {open && !disabled && (
        <Popover anchor={anchor.current} onClose={close} testId="captions-menu" width={248}>
          <div className="tx-pop-label">{editorText("Show")}</div>
          <div className="tx-modes" role="radiogroup">
            {existing && !style.startsWith('caption:') && <button role="radio" aria-checked={mode === undefined} className={mode === undefined ? 'active' : ''} disabled={busy} onClick={() => setMode(undefined)}>{editorText('Keep current')}</button>}
            {CAPTION_MODES.map((m) => (
              <button key={m} role="radio" aria-checked={mode === m} className={mode === m ? "active" : ""} data-testid={`captions-mode-${m}`} disabled={busy} onClick={() => setMode(m)}>
                {editorText(CAPTION_MODE_NAMES[m])}
              </button>
            ))}
          </div>
          <div className="tx-pop-label">{editorText("Style")}</div>
          <select className="field input" aria-label={editorText('Caption style')} data-testid="captions-look" value={style} disabled={busy} onChange={(e) => {
            const value = e.target.value;
            setStyle(value);
            const preset = value.startsWith('caption:') ? captionStyleDef(value.slice(8)) : undefined;
            if (preset) setMode(preset.mode ?? CAPTION_DEFAULTS.mode);
          }}>
            {existing && <option value="">{editorText('Keep current')}</option>}
            <optgroup label={editorText('Caption styles')}>
              {CAPTION_STYLES.map(preset => <option key={preset.id} value={`caption:${preset.id}`}>{editorText(preset.name)}</option>)}
            </optgroup>
            <optgroup label={editorText('Text styles')}>
              {Object.values(TEXT_STYLES).map((look) => <option key={look.id} value={look.id}>{editorText(look.name)}</option>)}
            </optgroup>
          </select>
          <button
            className="btn primary tx-pop-go"
            data-testid="captions-make"
            disabled={busy||!preview||'problem' in preview}
            aria-busy={busy}
            onClick={() => void make()}
          >
            <Captions />
            {fixesDropped?editorText('Remake (reset {0} corrections)',{0:fixesDropped}):existing ? editorText("Remake captions") : editorText("Make captions")}
          </button>
          {preview&&'problem' in preview&&<div className="inline-error">{editorText(preview.problem)}</div>}
        </Popover>
      )}
    </div>
  );
}

export function Untranscribed({ mediaId, name, online, whisperReady, checking, modelMissing, modelLabel }: { mediaId: string; name: string; online: boolean; whisperReady: boolean; checking: boolean; modelMissing: boolean; modelLabel: string }) {
  useEditorLanguage();
  const app = useApp();
  const job = activeAnalyses(app, mediaId).find((j) => j.analyzer === "transcript");
  const starting=app.analysisStarting?.includes(mediaId)===true;
  if (job) {
    const pct = Math.round((job.phase === "running" ? job.progress : 0) * 100);
    return (
      <div className="tx-state busy" data-testid="tx-progress" data-phase={job.phase}>
        <div className="tx-state-row">
          <span className="tx-state-text">{analysisText(job)}</span>
          <Tooltip label={editorText("Stop transcribing")}>
            <button className="icon-btn" data-testid="tx-cancel" aria-label={editorText("Stop transcribing")} disabled={!online||!app.biliupCanEdit||app.analysisCancelling?.includes(job.id)} onClick={() => void actions.cancelAnalysis(job.id)}>
              <X />
            </button>
          </Tooltip>
        </div>
        <div className="tx-bar" role="progressbar" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100}>
          <span style={{ width: `${pct}%` }} />
        </div>
      </div>
    );
  }
  if (!checking && !whisperReady) {
    return (
      <div className="tx-state" data-testid="tx-no-whisper">
        <span className="tx-state-text">
          <strong>{name}</strong> {editorText(" isn't transcribed. Transcribing needs whisper.cpp, which isn't installed.")}</span>
        <button className="btn" onClick={actions.openSetup}>
          <Settings />
          {editorText("Set up")}</button>
      </div>
    );
  }
  return (
    <div className="tx-state" data-testid="tx-untranscribed">
      <span className="tx-state-text">
          <strong>{name}</strong>{editorText(" isn't transcribed yet.")}</span>
      <div className="tx-state-row">
        <button className="btn primary" data-testid="tx-transcribe" aria-busy={starting} title={modelMissing?editorText('The first transcript downloads {0} on the server.',{0:modelLabel}):undefined} disabled={!online || !app.biliupCanEdit || checking || starting || app.transcriptionPending} onClick={() => void actions.transcribe(mediaId)}>
          <Captions />
          {editorText(starting?'Working…':modelMissing?'Download and transcribe':'Transcribe')}</button>
        {modelMissing && (
          <button className="link-btn" data-testid="tx-setup" onClick={actions.openSetup}>
            {editorText("Speech model settings")}</button>
        )}
      </div>
    </div>
  );
}

/**
 * Marks the word under the playhead (a DOM attribute, so a playing video doesn't re-render every
 * word 60 times a second) and keeps it in view while playing.
 */
export function PlayheadWord({ words, container }: { words: ReadonlyArray<EditWord>; container: React.RefObject<HTMLDivElement | null> }) {
  useEditorLanguage();
  // The current word's key: re-renders this component only when it changes, not every frame.
  const key = usePlayback((s) => {
    const frame = Math.floor(s.frame);
    // Words are in edit order with increasing starts: the last one starting at or before the frame.
    let lo = 0;
    let hi = words.length - 1;
    let at = -1;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (words[mid]!.start <= frame) (at = mid), (lo = mid + 1);
      else hi = mid - 1;
    }
    return at >= 0 && frame < words[at]!.end ? wordKey(words[at]!) : undefined;
  });
  useEffect(() => {
    const root = container.current;
    if (!root) return;
    root.querySelector("[data-current]")?.removeAttribute("data-current");
    if (!key) return;
    const el = root.querySelector<HTMLElement>(`[data-key="${CSS.escape(key)}"]`);
    if (!el) return;
    el.dataset.current = "true";
    if (!getPlayback().playing) return;
    const box = root.getBoundingClientRect();
    const r = el.getBoundingClientRect();
    if (r.top < box.top || r.bottom > box.bottom) el.scrollIntoView({ block: "nearest" });
  }, [key, words, container]);
  return null;
}
