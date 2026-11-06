import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Timeline | Mixer (D32, D83): the bottom panel's modes (split out of Timeline.tsx). The
// Transcript is a tab of the left rail (U11, D177).
import { playback, usePlayback } from "@/app/ui/editor/preview/playback";

export function BottomTabs() {
  useEditorLanguage();
  const bottom = usePlayback((s) => s.bottom);
  return (
    <div className="seg" data-testid="bottom-tabs">
      <button className={bottom === "timeline" ? "active" : ""} data-testid="bottom-timeline" onClick={() => playback.setBottom("timeline")}>
        {editorText("Timeline")}</button>
      <button className={bottom === "mixer" ? "active" : ""} data-testid="bottom-mixer" title={editorText("Volume, pan, effects, ducking, mute and solo per track; the master volume, limiter and loudness target; live meters")} onClick={() => playback.setBottom("mixer")}>
        {editorText("Mixer")}</button>
    </div>
  );
}
