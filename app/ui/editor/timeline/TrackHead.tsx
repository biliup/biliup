import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// A track's head on the left of the timeline (split out of Timeline.tsx): its name (double-click
// renames it in place), the main track's magnet, volume, mute, hide and lock; right-click opens
// the track menu (T7).
import { AudioLines, Eye, EyeOff, Film, Layers, Lock, LockOpen, Magnet, Type, Volume2, VolumeX } from "lucide-react";
import React from "react";

import { soundTrack } from "@/app/ui/editor/audio/sound";
import type { ProjectModel, Track } from "@/app/lib/editor/state/project";
import { actions } from "@/app/lib/editor/state/store";
import { trackFlag } from "@/app/ui/editor/timeline/edit";
import { RenameField } from "@/app/ui/editor/timeline/RenameField";
import { TrackVolume } from "@/app/ui/editor/timeline/TrackVolume";

export function TrackHead({ project, track, online, renaming, onRenamed, onMenu, onRename }: { project: ProjectModel; track: Track; online: boolean; renaming: boolean; onRenamed: () => void; onMenu: (e: React.MouseEvent) => void; onRename: () => void }) {
  useEditorLanguage();
  const main = track.role === "main";
  const Icon = track.kind === "text" ? Type : track.kind === "audio" ? AudioLines : main ? Film : Layers;
  const audio = track.kind === "audio";
  const hidden = trackFlag(track, "hidden");
  const muted = trackFlag(track, "mute");
  const locked = trackFlag(track, "locked");
  return (
    <div
      className={`tl-head ${main ? "main" : ""}`}
      data-testid="track"
      data-id={track.id}
      data-kind={track.kind}
      data-main={main ? "true" : undefined}
      data-hidden={hidden ? "true" : undefined}
      data-muted={muted ? "true" : undefined}
      data-locked={locked ? "true" : undefined}
      onContextMenu={onMenu}
    >
      <div className="name" onDoubleClick={() => online && !locked && onRename()}>
        <Icon />
        {renaming ? (
          <RenameField testId="track-rename" value={track.name} onDone={(name,origin) => (name !== undefined && name !== track.name && void actions.renameTrack(track.id, name,origin), onRenamed())} />
        ) : (
          <span className="ellipsis" data-testid="track-name">
            {track.id.startsWith('biliup-')?editorText(track.name):track.name}
          </span>
        )}
        {main && (
          <span className="badge" title={editorText("Main track: clips play back to back")}>
            <Magnet />
          </span>
        )}
      </div>
      {(audio || soundTrack(project, track)) && <TrackVolume project={project} track={track} online={online} />}
      {(audio || soundTrack(project, track)) && (
        <button className={`icon-btn ${muted ? "on" : ""}`} data-testid="track-mute" aria-pressed={muted} disabled={!online} title={muted ? editorText("Unmute") : audio ? editorText("Mute") : editorText("Mute the clips' sound")} onClick={() => void actions.toggleTrack(track.id, "mute")}>
          {muted ? <VolumeX /> : <Volume2 />}
        </button>
      )}
      {!audio && (
        <button className={`icon-btn ${hidden ? "on" : ""}`} data-testid="track-hide" aria-pressed={hidden} disabled={!online} title={hidden ? editorText("Show in the preview") : editorText("Hide from the preview")} onClick={() => void actions.toggleTrack(track.id, "hidden")}>
          {hidden ? <EyeOff /> : <Eye />}
        </button>
      )}
      <button className={`icon-btn ${locked ? "on" : ""}`} data-testid="track-lock" aria-pressed={locked} disabled={!online} title={locked ? editorText("Unlock") : editorText("Lock: no edits by hand or by the agent")} onClick={() => void actions.toggleTrack(track.id, "locked")}>
        {locked ? <Lock /> : <LockOpen />}
      </button>
    </div>
  );
}
