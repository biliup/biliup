import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Inspector's sound editor (D83): volume, fades, beats.
import { MAX_DB, SILENCE_DB, soundMedia } from "@/app/lib/editor/core/audio/audio";
import type { Item } from "@/app/lib/editor/core/timeline/types";
import { Activity, MoveHorizontal, X } from "lucide-react";

import { type ProjectModel } from "@/app/lib/editor/state/project";
import { useBeats } from "@/app/ui/editor/audio/beats";
import { analysisText } from "@/app/lib/editor/state/analysisText";
import { actions, activeAnalyses, useApp } from "@/app/lib/editor/state/store";
import { dbLabel, planFade, planFitSoundToVideo, planGain, transitionFades } from "@/app/ui/editor/audio/sound";
import { Slider, Group } from "@/app/ui/editor/inspector/fields";
import { NumInput } from "@/app/ui/editor/inspector/controls";
import {useProjectEdit} from '../workspace/projectEdit';
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import { beatsInItem } from '../timeline/edit/snap';

export function AudioEditor({ project, item, locked }: { project: ProjectModel; item: Item; locked: boolean }) {
  useEditorLanguage();
  const fps = project.canvas.fps;
  const db = item.gainDb ?? 0;
  const edit=useProjectEdit();
  const follows = transitionFades(project, item.id);
  const transitionHint = 'This fade follows the video transition. Edit the transition at the cut.';
  const secs = (frames: number | undefined) => (frames ?? 0) / fps;
  return (
    <div data-testid="audio-editor">
      <Group label={editorText("Volume")}>
        <div className="slider-row">
          <Slider
            testId="insp-volume"
            value={db}
            min={SILENCE_DB}
            max={MAX_DB}
            step={0.5}
            title={editorText("{0} dB", { 0: dbLabel(db) })}
            disabled={locked}
            onPreview={v=>edit.preview(project,planGain(project,item.id,v))}
            onCancel={edit.cancel}
            onCommit={(v,origin)=>edit.commit(p=>planGain(p,item.id,v),origin)}
          />
          <NumInput label={editorText("dB")} testId="insp-volume-num" value={db} step={0.5} min={SILENCE_DB} max={MAX_DB} disabled={locked} clearable onPreview={v => edit.preview(project, planGain(project, item.id, v))} onCancel={edit.cancel} onCommit={(v, origin) => { void edit.commit(p => planGain(p, item.id, v ?? 0), origin); }} />
        </div>
      </Group>
      <Group label={editorText("Fades")}>
        <div className="field-row">
          <NumInput label={editorText("In s")} testId="insp-fade-in" value={follows.in ? undefined : secs(item.fadeIn)} placeholder={follows.in ? editorText('Transition') : undefined} title={follows.in ? transitionHint : undefined} step={1 / fps} min={0} max={Math.max(0, item.duration - (item.fadeOut ?? 0)) / fps} disabled={locked || follows.in} clearable onPreview={v => edit.preview(project, planFade(project, item.id, 'in', Math.round(v * fps)))} onCancel={edit.cancel} onCommit={(v, origin) => { void edit.commit(p => planFade(p, item.id, 'in', Math.round((v ?? 0) * fps)), origin); }} />
          <NumInput label={editorText("Out s")} testId="insp-fade-out" value={follows.out ? undefined : secs(item.fadeOut)} placeholder={follows.out ? editorText('Transition') : undefined} title={follows.out ? transitionHint : undefined} step={1 / fps} min={0} max={Math.max(0, item.duration - (item.fadeIn ?? 0)) / fps} disabled={locked || follows.out} clearable onPreview={v => edit.preview(project, planFade(project, item.id, 'out', Math.round(v * fps)))} onCancel={edit.cancel} onCommit={(v, origin) => { void edit.commit(p => planFade(p, item.id, 'out', Math.round((v ?? 0) * fps)), origin); }} />
        </div>
      </Group>
      {item.type === "audio" && (
        <div className="group">
          <button className="btn" data-testid="insp-fit-to-video" disabled={locked} title={editorText("Trim or loop to the end of the video, with a fade out")} onClick={() => void edit.commit(now => planFitSoundToVideo(now, item.id))}>
            <MoveHorizontal />
            {editorText("Fit to video")}</button>
        </div>
      )}
      {item.type === "clip" && (
        <div className="group">
          <button className="btn" data-testid="insp-detach-audio" disabled={locked} onClick={() => void actions.detachAudio(item.id)}>
            {editorText("Detach audio")}</button>
        </div>
      )}
      <BeatsGroup project={project} item={item} />
    </div>
  );
}

/**
 * The beats of the item's sound (D70): Find beats runs the analysis job (on demand, D66) with its
 * progress and a stop button; once cached, the tempo and the beat count. The timeline draws the
 * beats as ticks and snaps to them.
 */
function BeatsGroup({ project, item }: { project: ProjectModel; item: Item }) {
  useEditorLanguage();
  const app = useApp();
  const media = soundMedia(project, item);
  const entry = useBeats((s) => (media ? s.entries.get(media.id) : undefined));
  if (!media) return null;
  const online = app.connection === "connected" && !!app.biliupCanEdit;
  const job = activeAnalyses(app, media.id).find((j) => j.analyzer === "beats");
  return (
    <Group label={editorText("Beats")}>
      <div data-testid="insp-beats" data-state={job ? job.phase : entry === undefined ? "loading" : entry === null || entry === "failed" ? "none" : entry.bpm === null ? "no-beat" : "done"}>
        {job ? (
          <div className="beats-progress" title={analysisText(job)}>
            <Activity />
            <div className="bar">
              <span style={{ width: `${Math.round((job.phase === "running" ? job.progress : 0) * 100)}%` }} />
            </div>
            <Tooltip label={editorText("Stop finding the beats")}>
              <button className="icon-btn" data-testid="insp-beats-cancel" disabled={!online||app.analysisCancelling?.includes(job.id)} onClick={() => void actions.cancelAnalysis(job.id)}>
                <X />
              </button>
            </Tooltip>
          </div>
        ) : entry === null || entry === undefined || entry === "failed" ? (
          <button className="btn" data-testid="insp-find-beats" disabled={!online || entry === undefined || app.analysisStarting?.includes(media.id)} onClick={() => void actions.analyzeBeats(media.id)}>
            <Activity />
            {editorText("Find beats")}</button>
        ) : entry.bpm === null ? (
          <div className="small" data-testid="insp-beats-summary">{editorText("No steady beat found")}</div>
        ) : (
          <div className="small" data-testid="insp-beats-summary">
            {Math.round(entry.bpm * (item.speed ?? 1) * 10) / 10} {editorText(" BPM, ")}{beatsInItem(project, item, entry.beats).length} {editorText(" beats")}</div>
        )}
      </div>
    </Group>
  );
}

/**
 * A selected cut on the main track (D73): its transition's type (or none: a hard cut), length in
 * seconds and frames (the module's limits and the cut's: half the shorter clip, the footage past
 * the cut), and Remove (⌫ too). Each change is one edit through planSetTransition.
 */
