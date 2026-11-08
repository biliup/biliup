import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { SILENCE_DB } from "@/app/lib/editor/core/audio/audio";
import { trackFlag, planTrackFlag } from "@/app/ui/editor/timeline/edit";
import { planTrackPan, planTrackGain, dbLabel, planMasterGain, MASTER_EXPORT_NOTE } from "@/app/ui/editor/audio/sound";
import type { ProjectModel, Track } from "@/app/lib/editor/state/project";
import { EffectSlots, DuckRow, MasterControls } from "@/app/ui/editor/audio/mixer/effects";
import { PanSlider, Meter, Fader } from "@/app/ui/editor/audio/mixer/controls";
import {useProjectEdit} from '../../workspace/projectEdit';

export function Strip({ project, track, online }: { project: ProjectModel; track: Track; online: boolean }) {
  useEditorLanguage();
  const muted = trackFlag(track, "mute");
  const solo = trackFlag(track, "solo");
  const {commit,preview,cancel}=useProjectEdit();
  const db = track.mixer?.gainDb ?? 0;
  const pan = track.mixer?.pan ?? 0;
  return (
    <div className={`strip-ch ${muted ? "muted" : ""} ${solo ? "soloed" : ""}`} data-testid="mixer-strip" data-track={track.id}>
      <div className="strip-name ellipsis" title={track.name}>
        {track.name}
      </div>
      <EffectSlots project={project} track={track} online={online} />
      <DuckRow project={project} track={track} online={online} />
      <PanSlider
        name={track.name}
        value={pan}
        disabled={!online}
        onPreview={(p) => preview(project, planTrackPan(project, track.id, p))}
        onCommit={(p,origin) => void commit((now) => planTrackPan(now, track.id, p), origin)}
        onCancel={cancel}
      />
      <div className="strip-body">
        <Meter id={track.id} />
        <Fader
          testId="mixer-fader"
          label={editorText("{0} volume", { 0: track.name })}
          value={db}
          disabled={!online}
          onPreview={(v) => preview(project, planTrackGain(project, track.id, v))}
          onCommit={(v,origin) => void commit((now) => planTrackGain(now, track.id, v), origin)}
          onCancel={cancel}
        />
      </div>
      <div className="strip-foot">
        <span className="strip-db mono" data-testid="mixer-db">
          {dbLabel(db)}
        </span>
        <button className={`ms-btn ${muted ? "on mute" : ""}`} data-testid="mixer-mute" aria-pressed={muted} disabled={!online} title={muted ? editorText("Unmute") : editorText("Mute")} onClick={() => void commit((now)=>planTrackFlag(now,track.id,'mute'))}>
          {editorText("M")}</button>
        <button
          className={`ms-btn ${solo ? "on solo" : ""}`}
          data-testid="mixer-solo"
          aria-pressed={solo}
          disabled={!online}
          title={solo ? editorText("Unsolo") : editorText("Solo: only soloed tracks play, in the preview and the export")}
          onClick={() => commit((now) => planTrackFlag(now, track.id, "solo"))}
        >
          {editorText("S")}</button>
      </div>
    </div>
  );
}

export function MasterStrip({ project, online }: { project: ProjectModel; online: boolean }) {
  useEditorLanguage();
  const {commit,preview,cancel}=useProjectEdit();
  const db = project.master?.gainDb ?? 0;
  return (
    <div className="strip-ch master" data-testid="mixer-master">
      <div className="strip-name">{editorText("Master")}</div>
      <MasterControls project={project} online={online} />
      <div className="pan-row" />
      <div className="strip-body">
        <Meter id="master" />
        <Fader
          testId="mixer-master-fader"
          label={editorText("Master volume")}
          value={db}
          disabled={!online}
          onPreview={(v) => preview(project, planMasterGain(project, v))}
          onCommit={(v,origin) => void commit((now) => planMasterGain(now, v), origin)}
          onCancel={cancel}
        />
      </div>
      <div className="strip-foot">
        <span className="strip-db mono" data-testid="mixer-master-db">
          {dbLabel(db)}
        </span>
      </div>
      {db !== 0 && (
        <div className="master-note small" data-testid="mixer-master-note" title={MASTER_EXPORT_NOTE}>
          {db <= SILENCE_DB ? editorText("Silent: the export too") : editorText("Export: As mixed")}
        </div>
      )}
    </div>
  );
}
