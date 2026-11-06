import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The bar under the preview: timecode, the transport buttons, the canvas, the preview's zoom and
// hand (G6) and the full-window preview (G7b). Split out of Preview.tsx.
// Issue #62 (D233): the bar is always 40px tall and never clips. As the preview gets narrow the
// less important parts go first (styles.css, @container stage): the canvas label, the end time,
// the jumps to start and end, the zoom and the frame steps, then the hand, the area and the time itself.
import { Hand, Maximize, SquareDashedMousePointer, Minimize, Pause, Play, SkipBack, SkipForward, StepBack, StepForward } from "lucide-react";

import { canvasLabel, frameTimecode } from "@/app/lib/editor/state/format";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import { layoutOf, type ProjectModel } from "@/app/lib/editor/state/project";
import { previewPlayback as playback, usePreviewPlayback } from "@/app/ui/editor/preview/playback";
import { keysOf } from '@/app/ui/editor/workspace/keymap';
import { ZOOM_STEPS, type useViewport } from "@/app/ui/editor/preview/viewport";

export function Transport({ project, view, full, onFull, region, onRegion }: { project: ProjectModel; view: ReturnType<typeof useViewport>; full: boolean; onFull: () => void; region: boolean; onRegion?: () => void }) {
  useEditorLanguage();
  const fps = project.canvas.fps;
  const frame = usePreviewPlayback((s) => Math.floor(s.frame));
  const playing = usePreviewPlayback((s) => s.playing);
  const { end, frames } = layoutOf(project);
  const custom = view.zoom !== "fit" && !ZOOM_STEPS.includes(view.zoom);
  return (
    <div className="transport">
      <span className="timecode" data-testid="timecode">
        {frameTimecode(frame, fps)}
        <span className="tc-end"> / {frameTimecode(end, fps)}</span>
      </span>
      <div className="transport-mid">
        {/* #86 (D239): tooltips with the keys from the keymap. */}
        <Tooltip label={editorText("Go to start")} shortcut="toStart">
          <button className="icon-btn tp-jump" data-testid="to-start" disabled={frame === 0} onClick={playback.toStart}>
            <SkipBack />
          </button>
        </Tooltip>
        <Tooltip label={editorText("Back one frame")} shortcut="frameBack">
          <button className="icon-btn tp-step" data-testid="step-back" disabled={frame === 0} onClick={() => playback.step(-1)}>
            <StepBack />
          </button>
        </Tooltip>
        <Tooltip label={playing ? editorText("Pause") : editorText("Play")} shortcut="play">
          <button className="icon-btn play" data-testid="play" disabled={frames <= 1} onClick={playback.toggle}>
            {playing ? <Pause /> : <Play />}
          </button>
        </Tooltip>
        <Tooltip label={editorText("Forward one frame")} shortcut="frameForward">
          <button className="icon-btn tp-step" data-testid="step-forward" disabled={frame >= Math.max(frames - 1, end)} onClick={() => playback.step(1)}>
            <StepForward />
          </button>
        </Tooltip>
        <Tooltip label={editorText("Go to end")} shortcut="toEnd">
          <button className="icon-btn tp-jump" data-testid="to-end" disabled={frame >= end} onClick={playback.toEnd}>
            <SkipForward />
          </button>
        </Tooltip>
      </div>
      <span className="muted small" data-testid="canvas-info">
        {canvasLabel(project.canvas)}
      </span>
      <select
        className="zoom-select"
        data-testid="preview-zoom"
        title={`${editorText('Preview zoom')} (${keysOf('zoomPointer').join(' / ')})`}
        value={view.zoom === "fit" ? "fit" : custom ? "custom" : String(view.zoom)}
        onChange={(e) => view.setZoom(e.target.value === "fit" ? "fit" : Number(e.target.value))}
      >
        <option value="fit">{editorText("Fit")}</option>
        {ZOOM_STEPS.map((z) => (
          <option key={z} value={String(z)}>
            {Math.round(z * 100)}%
          </option>
        ))}
        {custom && <option value="custom">{Math.round(view.scale * 100)}%</option>}
      </select>
      {/* Issue #118: point the agent at part of the picture. */}
      <Tooltip label={editorText("Area: drag over the picture to point the agent at it (at this frame, or In to Out)")}>
        <button className={`icon-btn tp-region ${region ? "on" : ""}`} data-testid="preview-region" aria-pressed={region} disabled={frames <= 1 || !onRegion} onClick={onRegion}>
          <SquareDashedMousePointer />
        </button>
      </Tooltip>
      <Tooltip label={editorText("Hand: drag to pan the preview")}>
        <button className={`icon-btn tp-hand ${view.hand ? "on" : ""}`} data-testid="preview-hand" aria-pressed={view.hand} onClick={() => view.setHand(!view.hand)}>
          <Hand />
        </button>
      </Tooltip>
      <Tooltip label={full ? editorText("Exit full screen") : editorText("Full screen preview")} shortcut={full ? "exitFull" : undefined}>
        <button className="icon-btn" data-testid="preview-fullscreen" aria-pressed={full} onClick={onFull}>
          {full ? <Minimize /> : <Maximize />}
        </button>
      </Tooltip>
    </div>
  );
}
