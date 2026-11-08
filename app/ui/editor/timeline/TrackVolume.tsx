import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { SlidersHorizontal } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { MAX_DB, SILENCE_DB } from "@/app/lib/editor/core/audio/audio";

import { dbLabel, planTrackGain } from "@/app/ui/editor/audio/sound";
import { NumberField, Slider } from "@/app/ui/editor/inspector/fields";
import type { ProjectModel, Track } from "@/app/lib/editor/state/project";
import {useApp} from '@/app/lib/editor/state/store';
import {useProjectEdit} from '../workspace/projectEdit';
import type {GestureOrigin} from '../workspace/pointerGesture';
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";

/**
 * A track's volume (D54): a button in its header opens a slider (−∞…+12 dB) and a number. The
 * slider is heard while it moves (a local draft the playback mix plays) and saved on release as
 * one edit; the mixer's mute and the rest stay as they are.
 */
export function TrackVolume({ project, track, online }: { project: ProjectModel; track: Track; online: boolean }) {
  useEditorLanguage();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const db = track.mixer?.gainDb ?? 0;
  const edit=useProjectEdit();
  const editable=online&&!!useApp().biliupCanEdit;
  useEffect(() => {
    if (!open) return;
    const close = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener("pointerdown", close);
    return () => window.removeEventListener("pointerdown", close);
  }, [open]);
  const send=(v:number,origin?:GestureOrigin)=>edit.commit(p=>planTrackGain(p,track.id,v),origin);
  return (
    <div className="track-volume" ref={ref}>
      <Tooltip label={editorText("Track volume: {0} dB", { 0: dbLabel(db) })}>
        <button
          className={`icon-btn ${db !== 0 ? "on-soft" : ""}`}
          data-testid="track-volume"
          aria-expanded={open}
          disabled={!editable}
         
          onClick={() => setOpen(!open)}
        >
          <SlidersHorizontal />
        </button>
      </Tooltip>
      {open && (
        <div className="track-volume-pop" data-testid="track-volume-pop">
          <div className="group-label">{track.name} {editorText(" volume")}</div>
          <div className="slider-row">
            <Slider
              testId="track-volume-slider"
              value={db}
              min={SILENCE_DB}
              max={MAX_DB}
              step={0.5}
              disabled={!editable}
              onPreview={v=>edit.preview(project,planTrackGain(project,track.id,v))}
              onCancel={edit.cancel}
              onCommit={send}
            />
            <NumberField label={editorText("dB")} testId="track-volume-num" value={db} step={0.5} min={SILENCE_DB} max={MAX_DB} disabled={!editable} onCommit={(v,origin) => send(v ?? 0,origin)} />
          </div>
          <div className="hint-line muted small">{db <= SILENCE_DB ? editorText("Silent") : editorText("{0} dB on every item of the track", { 0: dbLabel(db) })}</div>
        </div>
      )}
    </div>
  );
}
