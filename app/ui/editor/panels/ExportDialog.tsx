import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { isTextComposition } from '@/app/lib/editor/keyboard';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Export is a job on the current biliup deployment. Closing the dialog keeps it running;
// completed files are downloaded through the same authenticated API.
// Subtitle downloads remain available when the server cannot encode video.
// Subtitles (D105), when the project has captions or a Subtitles track: burn them in (default, as
// before), a .srt / .vtt next to the video, a soft subtitle track, or just the file.
// Motion blur (D291), when the project has graphics: on by default unless Draft (or as the project
// says); the preview draws no blur.
import type { ExportLoudness, ExportQuality, ExportSubtitles } from "@/app/lib/editor/contracts";
import { mixNote, SILENCE_DB } from "@/app/lib/editor/core/audio/audio";
import { dbWords } from "@/app/lib/editor/core/timeline/edit-log";
import { CODEC_NAMES, defaultEncoder, exportFrameRates, exportShortSides, outputSize, presetBitrateMbps, type ExportCodec, type ExportEncoder } from "@/app/lib/editor/core/export";
import { choiceOfTarget, masterLimiter, projectTargetLufs } from "@/app/lib/editor/core/audio/effects";
import { SUBTITLE_LANGUAGES, subtitleCues, type SubtitleFormat } from "@/app/lib/editor/core/text/subtitles";
import { subtitlesForExport } from "@/app/lib/editor/export/subtitles";
import { motionBlurDefault, projectMotionBlur } from "@/app/lib/editor/core/visual/motionBlur";
import { ChevronDown, ChevronRight, X } from "lucide-react";
import { useMemo, useState } from "react";

import { CheckRow } from "@/app/ui/editor/onboarding/rows";
import { actions, useApp, type Project } from "@/app/lib/editor/state/store";
import { frameTimecode } from "@/app/lib/editor/state/format";
import { layoutOf } from "@/app/lib/editor/state/project";
import { usePlayback } from "@/app/ui/editor/preview/playback";
import { NumberField } from "@/app/ui/editor/inspector/fields";
import { effectiveRange, type RangeKind } from "@/app/ui/editor/panels/exportRange";
import { exportSpans } from "@/app/ui/editor/panels/exportSpans";
import { loudnessCards, fileName, baseName, fpsText, eta, QUALITIES, ENCODER_NAMES, RANGE_KINDS } from "@/app/ui/editor/panels/exportDialog/format";
import {exportRunning,preparingStatus} from '@/app/lib/editor/export/jobs';
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import {GlobalDialog} from '@/app/ui/editor/controls/GlobalDialog';
import {useEditorAction} from '@/app/ui/editor/controls/useEditorAction';
import {apiResourceUrl} from '@/app/lib/renders';
import Link from 'next/link';
import {outputHref} from '@/app/lib/outputs';

export function ExportDialog({ project, dir }: { project: Project; dir: string }) {
  const editorLanguage=useEditorLanguage();
  const { snapshot, connection, biliupCanEdit, biliupSaving, biliupDirty, biliupSaveConflict, exportSubmitting, exportJobs, exportJobId, exportJobsLoaded, exportJobsError, setupChecks, setupChecking } = useApp();
  const {run,busy:localBusy,error,setError}=useEditorAction();
  const canSubmit = connection === 'connected' && !!biliupCanEdit && snapshot?.dir === dir && !biliupSaving && !biliupDirty && !biliupSaveConflict;
  const [quality, setQuality] = useState<ExportQuality>("standard");
  // F4 (D171): the Advanced row (collapsed): codec, bitrate, size, frame rate; undefined = the preset's.
  const [advanced, setAdvanced] = useState(false);
  const [codec, setCodec] = useState<ExportCodec>("h264");
  const [bitrate, setBitrate] = useState<number | undefined>();
  // D212: undefined = the preset's (High software, Draft and Standard hardware).
  const [encoder, setEncoder] = useState<ExportEncoder | undefined>();
  const [shortSide, setShortSide] = useState<number | undefined>();
  const [outFps, setOutFps] = useState<number | undefined>();
  // The default comes from the project's loudness target (D87: master.loudness.targetLufs, −14 when unset).
  const target = projectTargetLufs(project);
  const [loudness, setLoudness] = useState<ExportLoudness>(() => choiceOfTarget(target));
  const LOUDNESS = loudnessCards(target);
  const [name, setName] = useState(fileName(project.name));
  const busy = localBusy || !!exportSubmitting;
  // Subtitles (D105): what the project's captions and Subtitles tracks would export as.
  const totalCues = useMemo(() => subtitleCues(project).length, [project]);
  const [burn, setBurn] = useState(true);
  const [subFile, setSubFile] = useState(false);
  const [subTrack, setSubTrack] = useState(false);
  const [subFormat, setSubFormat] = useState<SubtitleFormat>("srt");
  const [language, setLanguage] = useState(editorLanguage==='zh-CN'?'zh':'en');
  const [savedSubs, setSavedSubs] = useState<string | undefined>();
  // D291: motion blur on graphics; undefined = the default for the quality (and the project's setting).
  const hasGraphics = useMemo(() => project.tracks.some((t) => !t.hidden && t.items.some((i) => i.type === "graphic")), [project]);
  const [blurChoice, setBlurChoice] = useState<boolean | undefined>();
  const blurOn = blurChoice ?? motionBlurDefault(project, quality);
  const blurLook = projectMotionBlur(project);
  // A finished job stays on screen until "Export again" or the dialog closes.
  const [dismissed, setDismissed] = useState<string | undefined>();
  const job = exportJobs.find((j) => j.id === exportJobId && j.projectDir === dir && j.id !== dismissed);
  const awaitingJob = !!exportJobId && exportJobId!==dismissed && !exportJobs.some(j => j.id === exportJobId);
  const ffmpeg = setupChecks?.find((c) => c.id === "ffmpeg");
  const noFfmpeg = ffmpeg !== undefined && ffmpeg.status !== "ready" && !job;

  // G7 (D127): the part to export.
  const fps = project.canvas.fps;
  const end = layoutOf(project).end;
  const markIn = usePlayback((x) => x.markIn);
  const markOut = usePlayback((x) => x.markOut);
  const selectedId = usePlayback((x) => x.selected);
  const also = usePlayback((x) => x.also);
  const { marks, selection } = exportSpans(project, { markIn, markOut, selected: selectedId, also });
  const [rangeKind, setRangeKind] = useState<RangeKind>("all");
  const [custom, setCustom] = useState<{ from: number; to: number }>(() => marks ?? { from: 0, to: end });
  // Marks or a selection that went away (an undo, the agent) fall back to the whole timeline, visibly.
  const effective = effectiveRange(rangeKind, { marks, selection, custom });
  const range = effective.range;
  const subtitlePlan = useMemo(() => subtitlesForExport(project, range), [project, range?.from, range?.to]);
  const rangeProblem = "problem" in subtitlePlan ? editorText(subtitlePlan.problem) : undefined;
  const cueCount = "problem" in subtitlePlan ? 0 : subtitlePlan.cues.length;
  const rangeOk = end > 0 && !rangeProblem;
  const subtitlesMissing = totalCues > 0 && (subFile || subTrack) && cueCount === 0;

  const output = fileName(name);
  const start = () => {
    if (!canSubmit || !snapshot || !name.trim() || !rangeOk || busy || noFfmpeg || subtitlesMissing) return;
    const subtitles: ExportSubtitles | undefined = cueCount && (subFile || subTrack) ? { ...(subFile ? { file: subFormat } : {}), ...(subTrack ? { track: true, language } : {}) } : undefined;
    const video = { ...(codec !== "h264" ? { codec } : {}), ...(encoder !== undefined ? { encoder } : {}), ...(bitrate !== undefined ? { bitrateMbps: bitrate } : {}), ...(shortSide !== undefined ? { shortSide } : {}), ...(outFps !== undefined ? { fps: outFps } : {}) };
    void run(async()=>{
      const refused=await actions.startExport({output,quality,loudness,...(Object.keys(video).length?{video}:{}),...(loudness==='target'?{targetLufs:target}:{}),...(subtitles?{subtitles}:{}),...(cueCount&&!burn?{burnCaptions:false}:{}),...(range?{range}:{}),...(hasGraphics?{motionBlur:blurOn}:{})},{dir,baseRevision:snapshot.revision});
      if(refused)throw new Error(refused.message);
    });
  };
  /** Download just this range's subtitles, starting at the exported video's frame zero. */
  const subtitlesOnly = () => {
    if (!canSubmit || !snapshot || !name.trim() || !rangeOk || !cueCount || busy) return;
    setSavedSubs(undefined);
    const path = output.replace(/\.mp4$/i, `.${subFormat}`);
    void run(async()=>{
      const result=await actions.exportSubtitles({output:path,...(range?{range}:{})},{dir,baseRevision:snapshot.revision});
      if('message'in result)throw new Error(result.message);
      return result;
    },{onSuccess:result=>setSavedSubs(editorText('Downloaded {0} ({1} cues).',{0:baseName(result.path),1:result.cues}))});
  };

  const changeJob=(action:'cancel'|'retry')=>{
    if(!job||busy)return;
    void run(async()=>{
      const refused=await(action==='cancel'?actions.cancelExport(job.id):actions.retryExport(job.id));
      if(refused)throw new Error(refused.message);
    });
  };
  const refreshJobs=()=>void run(async()=>{
    const result=await actions.refreshExportJobs();
    if(!result.ok)throw new Error(result.error.message);
  },{readOnly:true});
  const statusError=exportJobsError&&<div className="inline-error" role="alert">{editorText('Export status could not be refreshed: {0}',{0:exportJobsError})} <button type="button" className="btn" disabled={busy} onClick={refreshJobs}>{editorText('Retry')}</button></div>;
  let body;
  if (job) {
    const pct = Math.round(job.progress * 100);
    const preparing = preparingStatus(job);
    const status =
      job.phase === "queued"
        ? editorText("Queued")
        : job.phase === "preparing"
          ? preparing.text
          : job.phase === "rendering"
            ? `${editorText('Exporting…')} ${pct}%`
            : job.phase === "done"
              ? editorText('Completed')
              : editorText(job.phase==='cancelled'?'Cancelled':"The export didn't finish.");
    body = (
      <div data-testid="export-job" data-phase={job.phase} data-progress={job.progress} data-step={job.phase === "preparing" ? job.step?.kind : undefined} data-output={job.output}>
        <p className="wiz-lead" data-testid="export-job-where" title={job.output}>
          <b className="mono">{baseName(job.output)}</b>
          , {job.width}×{job.height}, {fpsText(job.fps)}, {CODEC_NAMES[job.codec]}, {job.seconds.toFixed(1)} {editorText(" s")}{job.motionBlur ? editorText(", motion blur") : ""}
        </p>
        {(job.subtitlesFile || job.subtitleTrack || job.burnCaptions === false) && (
          <p className="wiz-meta" data-testid="export-job-subtitles">
            {[job.subtitlesFile ? editorText('Subtitles in {0}',{0:baseName(job.subtitlesFile)}) : "", job.subtitleTrack ? editorText('Subtitle track ({0})',{0:editorText(SUBTITLE_LANGUAGES.find((l) => l.tag === job.subtitleTrack)?.name ?? job.subtitleTrack)}) : "", job.burnCaptions === false ? editorText('Captions not burned in') : ""]
              .filter(Boolean)
              .join(", ")
              .replace(/^a/, "A")}
            .
          </p>
        )}
        <div className="export-status" data-testid="export-status">
          {status}
          {job.phase === "rendering" && <span className="muted">{eta(job.etaSeconds)}</span>}
          {job.phase === "preparing" && preparing.fraction !== undefined && <span className="muted">{Math.round(preparing.fraction * 100)}%</span>}
        </div>
        <div className="export-bar" data-testid="export-progress">
          <div style={{ width: `${job.phase === "done" ? 100 : job.phase === "rendering" ? pct : job.phase === "preparing" && preparing.fraction !== undefined ? Math.round(preparing.fraction * 100) : 0}%` }} className={(job.phase === "preparing" && preparing.fraction === undefined) || job.phase === "queued" ? "indeterminate" : ""} />
        </div>
        {job.warnings && job.warnings.length > 0 && job.phase !== "failed" && (
          <div className="export-warnings muted small" data-testid="export-warnings">
            {job.warnings.map((w) => (
              <div key={w}>{w}</div>
            ))}
          </div>
        )}
        {job.phase === "failed" && (
          <div className="inline-error" role="alert" data-testid="export-error">
            {job.error ?? editorText("Something went wrong.")}
          </div>
        )}
        {error&&error!==exportJobsError&&<p className="inline-error" role="alert">{error}</p>}
        {statusError}
        <div className="wiz-foot">
          <Link className="btn ghost" aria-disabled={busy} href={outputHref('editor',Number(job.id.slice('editor-export-'.length)))} onClick={event=>{if(busy)event.preventDefault();else actions.closeExport()}}>{editorText('Task details')}</Link>
          {exportRunning(job) ? (
            <>
              <button className="btn ghost" disabled={busy} onClick={actions.closeExport} title={editorText("The export keeps going")}>
                {editorText("Hide")}</button>
              <button className="btn" data-testid="export-cancel" disabled={busy||connection !== 'connected' || !biliupCanEdit} aria-busy={busy} onClick={() => void changeJob('cancel')}>
                {editorText("Cancel export")}</button>
            </>
          ) : (
            <>
              <button className="btn ghost" data-testid="export-again" disabled={busy} onClick={() => {setError('');setDismissed(job.id)}}>
                {job.phase === "done" ? editorText("Export again") : editorText("New export")}
              </button>
              {(job.phase==='failed'||job.phase==='cancelled')&&<button className="btn" disabled={busy||connection!=='connected'||!biliupCanEdit} aria-busy={busy} title={editorText('Retry the saved export version and settings')} onClick={()=>void changeJob('retry')}>{editorText('Retry')}</button>}
              {job.phase === "done" && <a className="btn" download href={apiResourceUrl(`/v1/editor/projects/${dir.slice(7)}/exports/${job.id.replace('editor-export-','')}/download?attachment=true`)}>{editorText('Download')}</a>}
              {job.phase === "done" && job.subtitlesFile && <a className="btn" download href={apiResourceUrl(`/v1/editor/projects/${dir.slice(7)}/exports/${job.id.replace('editor-export-','')}/download?attachment=true&subtitles=true`)}>{editorText('Download subtitles')}</a>}
              <button className="btn primary" data-testid="export-done" disabled={busy} onClick={actions.closeExport}>
                {editorText("Done")}</button>
            </>
          )}
        </div>
      </div>
    );
  } else if (awaitingJob) {
    body=<>
      {statusError||<div className="export-status" role="status">{editorText(exportJobsLoaded?'This export is no longer available.':'Loading export status…')}</div>}
      <div className="wiz-foot">
        <Link className="btn ghost" aria-disabled={busy} href={outputHref('editor',Number(exportJobId!.slice('editor-export-'.length)))} onClick={event=>{if(busy)event.preventDefault();else actions.closeExport()}}>{editorText('Task details')}</Link>
        <button className="btn" disabled={busy} onClick={()=>{setError('');setDismissed(exportJobId)}}>{editorText('New export')}</button>
      </div>
    </>;
  } else {
    const look = LOUDNESS.find((l) => l.id === loudness) ?? LOUDNESS[0]!;
    const limiter = masterLimiter(project);
    const note = mixNote(project);
    const masterDb = project.master?.gainDb ?? 0;
    body = (
      <form
        data-testid="export-form"
        onKeyDown={event=>{if(event.key==='Enter'&&isTextComposition(event.nativeEvent))event.preventDefault()}}
        onSubmit={(e) => {
          e.preventDefault();
          void start();
        }}
      >
        <fieldset className="export-options" disabled={busy || !canSubmit}>
        {noFfmpeg && (
          <div data-testid="export-no-ffmpeg">
            <div className="wiz-list"><CheckRow check={ffmpeg!} /></div>
            <button type="button" className="btn" data-testid="export-recheck" disabled={setupChecking} onClick={() => void actions.checkSetup()}>
              {setupChecking ? editorText("Checking…") : editorText("Check again")}
            </button>
          </div>
        )}
        <div className="wiz-cards">
          {QUALITIES.map((q) => {
            const size = outputSize(project.canvas, q.id, shortSide);
            const mbps = q.id === quality && bitrate !== undefined ? bitrate : codec === "hevc" ? presetBitrateMbps(q.id, codec, size, outFps ?? fps) : undefined;
            const enc = q.id === quality && encoder !== undefined ? encoder : defaultEncoder(q.id);
            return (
              <button type="button" key={q.id} className={`wiz-card ${q.id === quality ? "active" : ""}`} data-testid={`quality-${q.id}`} data-active={q.id === quality} data-codec={codec} onClick={() => setQuality(q.id)}>
                {editorText(q.label)}
                <span>
                  {size.width}×{size.height}, {fpsText(outFps ?? fps)}
                </span>
                <span data-testid={`quality-codec-${q.id}`}>
                  {CODEC_NAMES[codec]}, {mbps !== undefined ? editorText("{0} Mb/s", { 0: mbps }) : editorText(ENCODER_NAMES[enc])}
                </span>
                <span>{editorText(q.hint)}</span>
              </button>
            );
          })}
        </div>
        <button type="button" className="export-advanced-toggle" data-testid="export-advanced" aria-expanded={advanced} onClick={() => setAdvanced(!advanced)}>
          {advanced ? <ChevronDown /> : <ChevronRight />}
          {editorText("Advanced")}{!advanced && (codec !== "h264" || encoder !== undefined || bitrate !== undefined || shortSide !== undefined || outFps !== undefined) && <span className="muted">{[codec !== "h264" ? CODEC_NAMES[codec] : "", encoder !== undefined ? ENCODER_NAMES[encoder] : "", bitrate !== undefined ? `${bitrate} Mb/s` : "", shortSide !== undefined ? `${shortSide}p` : "", outFps !== undefined ? fpsText(outFps) : ""].filter(Boolean).join(", ")}</span>}
        </button>
        {advanced && (
          <div className="export-advanced" data-testid="export-advanced-body">
            <div className="export-adv-row">
              <span className="muted">{editorText("Codec")}</span>
              <div className="seg" data-testid="export-codec" role="radiogroup" aria-label={editorText("Codec")}>
                {(["h264", "hevc"] as const).map((c) => (
                  <button type="button" key={c} role="radio" aria-checked={codec === c} className={codec === c ? "active" : ""} data-testid={`export-codec-${c}`} onClick={() => setCodec(c)}>
                    {c === "h264" ? editorText("H.264") : editorText("HEVC (smaller, Apple devices)")}
                  </button>
                ))}
              </div>
            </div>
            <div className="export-adv-row">
              <span className="muted">{editorText("Encoder")}</span>
              <div className="seg" data-testid="export-encoder" role="radiogroup" aria-label={editorText("Encoder")}>
                {(["hardware", "software"] as const).map((e) => (
                  <button type="button" key={e} role="radio" aria-checked={(encoder ?? defaultEncoder(quality)) === e} className={(encoder ?? defaultEncoder(quality)) === e ? "active" : ""} data-testid={`export-encoder-${e}`} onClick={() => setEncoder(e === defaultEncoder(quality) ? undefined : e)}>
                    {e === "hardware" ? editorText("Hardware (faster)") : editorText("Software (smaller)")}
                  </button>
                ))}
              </div>
            </div>
            <div className="export-adv-row">
              <span className="muted">{editorText("Size")}</span>
              <select data-testid="export-size" value={shortSide ?? ""} onChange={(e) => setShortSide(e.target.value === "" ? undefined : Number(e.target.value))}>
                <option value="">{(({ width, height }) => `${width}×${height}`)(outputSize(project.canvas, quality))} ({editorText(QUALITIES.find((q) => q.id === quality)!.label)})</option>
                {exportShortSides(project.canvas).map((side) => {
                  const s = outputSize(project.canvas, quality, side);
                  return (
                    <option key={side} value={side}>
                      {s.width}×{s.height}
                    </option>
                  );
                })}
              </select>
              <span className="muted">{editorText("Frame rate")}</span>
              <select data-testid="export-fps" value={outFps ?? ""} onChange={(e) => setOutFps(e.target.value === "" ? undefined : Number(e.target.value))}>
                {exportFrameRates(fps).map((r, i) => (
                  <option key={r} value={i === 0 ? "" : r}>
                    {fpsText(r)}
                    {i === 0 ? editorText(" (project)") : ""}
                  </option>
                ))}
              </select>
            </div>
            <div className="export-adv-row">
              <span className="muted">{editorText("Bitrate")}</span>
              <NumberField label={editorText("Mb/s")} testId="export-bitrate" value={bitrate} placeholder={codec === "hevc" ? `${presetBitrateMbps(quality, codec, outputSize(project.canvas, quality, shortSide), outFps ?? fps)}` : editorText("Auto")} step={1} min={0.1} max={500} onCommit={(v) => setBitrate(v)} />
            </div>
          </div>
        )}
        <div className="wiz-list form-fields export-fields">
          <label className="wiz-field">
            <span>{editorText("File")}</span>
            <input data-testid="export-name" value={name} spellCheck={false} onChange={(e) => (setName(e.target.value), setError(''))} onBlur={() => setName(fileName(name))} />
          </label>
        </div>
        <div className="group-label export-label">{editorText("Range")}</div>
        <div className="seg export-range" data-testid="export-range" role="radiogroup" aria-label={editorText("Range")}>
          {RANGE_KINDS.map(([id, label]) => (
            <button
              type="button"
              key={id}
              role="radio"
              aria-checked={effective.kind === id}
              className={effective.kind === id ? "active" : ""}
              data-testid={`export-range-${id}`}
              disabled={(id === "marks" && !marks) || (id === "selection" && !selection)}
              onClick={() => (setRangeKind(id), id === "custom" && setCustom(range ?? custom))}
            >
              {editorText(label)}
            </button>
          ))}
        </div>
        {rangeKind === "custom" && (
          <div className="field-row export-custom">
            <NumberField label={editorText("From s")} testId="export-range-from" value={Number((custom.from / fps).toFixed(2))} step={0.1} min={0} onCommit={(v) => setCustom({ ...custom, from: Math.max(0, Math.round((v ?? 0) * fps)) })} />
            <NumberField label={editorText("To s")} testId="export-range-to" value={Number((custom.to / fps).toFixed(2))} step={0.1} min={0} onCommit={(v) => setCustom({ ...custom, to: Math.min(end, Math.round((v ?? 0) * fps)) })} />
          </div>
        )}
        {rangeOk&&<p className="wiz-meta" data-testid="export-range-text" data-from={range?.from ?? 0} data-to={range?.to ?? end}>
          {effective.fellBack && <span data-testid="export-range-fallback">{effective.fellBack === "marks" ? editorText("The In and Out marks are gone: whole timeline. ") : editorText("Nothing is selected now: whole timeline. ")}</span>}
          {editorText("{0} to {1}, {2} s", { 0: frameTimecode(range?.from ?? 0, fps), 1: frameTimecode(range?.to ?? end, fps), 2: (((range?.to ?? end) - (range?.from ?? 0)) / fps).toFixed(1) })}
        </p>}
        <div className="group-label export-label">{editorText("Loudness")}</div>
        <div className="wiz-cards">
          {LOUDNESS.map((l) => (
            <button type="button" key={l.id} className={`wiz-card ${l.id === loudness ? "active" : ""}`} data-testid={`loudness-${l.id}`} data-active={l.id === loudness} onClick={() => setLoudness(l.id)}>
              {editorText(l.label)}
              <span>{editorText(l.sub)}</span>
            </button>
          ))}
        </div>
        <p className="wiz-meta" data-testid="loudness-hint">
          {editorText(look.hint)}
          {limiter ? editorText(" Peaks stay under {0}{1} dBTP (the master limiter).", { 0: limiter.ceilingDb < 0 ? "−" : "", 1: Math.abs(limiter.ceilingDb) }) : editorText(" The master limiter is off: loud peaks can clip.")}
        </p>
        {note && (
          // D83: the export mixes what the preview plays, so a solo left on exports only those tracks.
          <p className="wiz-meta solo-note" data-testid="export-solo-note">
            {note}
          </p>
        )}
        {masterDb > SILENCE_DB && masterDb !== 0 && loudness !== "off" && (
          <p className="wiz-meta" data-testid="export-master-note">
            {editorText("The master volume (")}{dbWords(masterDb)}{editorText(") is heard in the editor and in As mixed; ")}{editorText(look.label)} {editorText(" evens the loudness out after it.")}</p>
        )}
        {hasGraphics && (
          <>
            <div className="group-label export-label">{editorText("Motion")}</div>
            <div className="export-subs" data-testid="export-motion">
              <label className="export-sub-row">
                <input type="checkbox" data-testid="export-motion-blur" checked={blurOn} onChange={(e) => setBlurChoice(e.target.checked)} />
                <span>{editorText("Motion blur")}</span>
                <span className="muted">
                  {editorText("Graphics: ")}{blurLook.samples} {editorText(" sub-frames a frame, a ")}{blurLook.shutter}{editorText("° shutter. The preview shows them sharp.")}</span>
              </label>
            </div>
          </>
        )}
        {totalCues > 0 && (
          <>
            <div className="group-label export-label">{editorText("Captions and subtitles")}</div>
            <div className="export-subs" data-testid="export-subtitles">
              <label className="export-sub-row">
                <input type="checkbox" data-testid="export-burn" checked={burn} onChange={(e) => setBurn(e.target.checked)} />
                <span>{editorText("Burn in captions")}</span>
                <span className="muted">{editorText("Drawn into the picture, as in the preview")}</span>
              </label>
              <label className="export-sub-row">
                <input type="checkbox" data-testid="export-sub-file" checked={subFile} onChange={(e) => setSubFile(e.target.checked)} />
                <span>{editorText("Subtitle file (.srt / .vtt)")}</span>
              </label>
              <label className="export-sub-row">
                <input type="checkbox" data-testid="export-sub-track" checked={subTrack} onChange={(e) => setSubTrack(e.target.checked)} />
                <span>{editorText("Soft subtitle track in the video")}</span>
                <select data-testid="export-sub-language" value={language} disabled={!subTrack} onChange={(e) => setLanguage(e.target.value)} title={editorText("The track's language")}>
                  {SUBTITLE_LANGUAGES.map((l) => (
                    <option key={l.code} value={l.code}>
                      {editorText(l.name)}
                    </option>
                  ))}
                </select>
              </label>
              <div className="export-sub-row">
                <span className="muted">{editorText("File format")}</span>
                <div className="seg" data-testid="export-sub-format">
                  {(["srt", "vtt"] as const).map((f) => (
                    <button type="button" key={f} className={f === subFormat ? "active" : ""} data-testid={`export-sub-format-${f}`} data-active={f === subFormat} onClick={() => setSubFormat(f)}>
                      {f === "srt" ? editorText("SRT") : editorText("WebVTT")}
                    </button>
                  ))}
                </div>
                <span className="muted" data-testid="export-sub-count">
                  {editorText('{0} cues',{0:cueCount})}</span>
              </div>
            </div>
            {!burn && !subFile && !subTrack && (
              <p className="wiz-meta" data-testid="export-no-captions-note">
                {editorText("The video will have no captions: pick a subtitles file or a soft track to keep them.")}</p>
            )}
            {savedSubs && (
              <p className="wiz-meta" data-testid="export-subtitles-saved">
                {savedSubs}
              </p>
            )}
          </>
        )}
        </fieldset>
        {rangeProblem && <div className="inline-error" role="alert">{rangeProblem}</div>}
        {!rangeProblem && subtitlesMissing && <div className="inline-error" role="alert">{editorText("No subtitles in this range.")}</div>}
        {error&&error!==exportJobsError&&<div className="inline-error" role="alert" data-testid="export-refused">{error}</div>}
        {statusError}
        <div className="wiz-foot">
          <button type="button" className="btn ghost" disabled={busy} onClick={actions.closeExport}>
            {editorText("Cancel")}</button>
          {totalCues > 0 && (
            <button type="button" className="btn" data-testid="export-subtitles-only" disabled={busy || !canSubmit || !name.trim() || !rangeOk || !cueCount} onClick={() => void subtitlesOnly()} title={editorText(cueCount ? "Just the subtitles as a .{0} file, no video" : "No subtitles in this range.", { 0: subFormat })}>
              {editorText("Subtitles only")}</button>
          )}
          <button type="submit" className="btn primary" data-testid="export-start" disabled={busy || !canSubmit || !name.trim() || !rangeOk || noFfmpeg || subtitlesMissing}>
            {editorText("Export")}</button>
        </div>
      </form>
    );
  }

  return (
    <GlobalDialog label={editorText('Export video')} onClose={actions.closeExport} busy={busy}>
      <div className="modal wiz" data-testid="export-modal">
        <div className="wiz-panel">
          <div className="export-head">
            <h1>{editorText("Export video")}</h1>
            <Tooltip label={editorText("Close")} keys={["Esc"]}>
              <button className="icon-btn" data-testid="export-close" disabled={busy} onClick={actions.closeExport}>
                <X />
              </button>
            </Tooltip>
          </div>
          {body}
        </div>
      </div>
    </GlobalDialog>
  );
}
