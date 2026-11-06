import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Browser capture shares the editor's recording, media and timeline pipeline.
import { Mic, Monitor, Speaker, Video } from "lucide-react";
import React, { useEffect, useState } from "react";
import { type MultiLayoutName, MULTI_LAYOUTS } from "@/app/lib/editor/core/timeline/place";

import { loadInputs, refreshAccess, openPreview, closePreview, startTake, useRecorder, type PartKind, type Resolution, type TakeOptions } from "@/app/ui/editor/capture/recorder";
import { useApp } from "@/app/lib/editor/state/store";
import { Composite } from "@/app/ui/editor/panels/capture/Composite";
import {watchCaptureAccess} from '../capture/access'
import {CaptureRecovery} from './capture/Recovery'

import { useSystemSound, useMonitor, Access, MicSelect, LevelMeter, Option, RecordBar, Picker } from "@/app/ui/editor/panels/capture/controls";
export { LevelMeter } from "@/app/ui/editor/panels/capture/controls";

/** The order sources show and record in. */
const SOURCES: ReadonlyArray<{ kind: PartKind; label: string; icon: typeof Monitor }> = [
  { kind: "screen", get label() { return editorText("Screen"); }, icon: Monitor },
  { kind: "voiceover", get label() { return editorText("Microphone"); }, icon: Mic },
  { kind: "camera", get label() { return editorText("Camera"); }, icon: Video },
  { kind: "system", get label() { return editorText("System audio"); }, icon: Speaker },
];
const PART_ORDER: ReadonlyArray<PartKind> = ["screen", "camera", "voiceover", "system"];
const LAYOUT_NAMES: Record<MultiLayoutName, string> = { tl: "Top left", tr: "Top right", bl: "Bottom left", br: "Bottom right", side: "Side by side" };

/** The recorder's source choices persist across visits. */
interface Settings {
  readonly on: Readonly<Record<PartKind, boolean>>;
  readonly camera: string;
  readonly mic: string;
  readonly resolution: Resolution;
  readonly layout: MultiLayoutName;
  readonly play: boolean;
  readonly mute: boolean;
  readonly place: boolean;
}
const KEY = "biliup.editor.capture";
const DEFAULTS: Settings = { on: { screen: true, voiceover: false, camera: false, system: false }, camera: "", mic: "", resolution: "1080p", layout: "br", play: true, mute: true, place: false };
let remembered: Settings | undefined;
function loadSettings(): Settings {
  if (remembered) return remembered;
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<Settings>;
    remembered = { ...DEFAULTS, ...raw, on: { ...DEFAULTS.on, ...(raw.on ?? {}) } };
  } catch {
    remembered = DEFAULTS;
  }
  return remembered;
}

const ok = (a: string | undefined) => a === "granted" || a === "unknown";

export function CaptureBrowser() {
  useEditorLanguage();
  const app = useApp();
  const phase = useRecorder((s) => s.phase);
  const message = useRecorder((s) => s.message);
  const recordedMs = useRecorder((s) => s.recordedMs);
  const offsets = useRecorder((s) => s.offsets);
  const access = useRecorder((s) => s.access);
  const cameras = useRecorder((s) => s.cameras);
  const mics = useRecorder((s) => s.mics);
  const system = useSystemSound();
  const [settings, setSettingsState] = useState<Settings>(loadSettings);
  const set = (patch: Partial<Settings>) =>
    setSettingsState((s) => {
      const next = { ...s, ...patch };
      remembered = next;
      try {
        localStorage.setItem(KEY, JSON.stringify(next));
      } catch {}
      return next;
    });
  useEffect(() => {
    const refresh=()=>void refreshAccess().then(loadInputs);
    refresh();
    return watchCaptureAccess(refresh);
  }, []);
  const busy = phase !== "idle";
  // Mid-take (back on the tab, or the agent's take) the toggles show what's recording.
  const runningParts = useRecorder((s) => s.parts);
  const runningMode = useRecorder((s) => s.mode);
  const running: Record<PartKind, boolean> | undefined = busy && runningMode
    ? (() => {
        const kinds: ReadonlyArray<PartKind> = runningParts ?? [runningMode === "multi" ? "screen" : runningMode];
        return { screen: kinds.includes("screen"), voiceover: kinds.includes("voiceover"), camera: kinds.includes("camera"), system: kinds.includes("system") };
      })()
    : undefined;
  const on = running ?? settings.on;
  const can: Record<PartKind, boolean> = { screen: ok(access?.screen), voiceover: ok(access?.microphone), camera: ok(access?.camera), system: system.can };
  const parts = PART_ORDER.filter((k) => on[k]);
  const mic = settings.mic && mics.some((m) => m.id === settings.mic) ? settings.mic : (mics[0]?.id ?? "");
  const camera = settings.camera && cameras.some((c) => c.id === settings.camera) ? settings.camera : (cameras[0]?.id ?? "");
  const screenOn = on.screen && can.screen;
  const cameraOn = on.camera && can.camera;
  const micOn = on.voiceover && can.voiceover;
  useMonitor(micOn && !busy, mic);
  useEffect(() => {
    if (!cameraOn) return;
    void openPreview(camera || undefined, settings.resolution);
    return () => closePreview();
  }, [cameraOn, camera, settings.resolution]);

  const ready: Record<PartKind, boolean> = { screen: can.screen, voiceover: can.voiceover, camera: can.camera, system: can.system };
  const allReady = parts.length > 0 && parts.every((k) => ready[k]);
  const canRecord = app.biliupCanEdit && app.connection === "connected" && !!app.snapshot && !busy && allReady;
  const video = on.screen || on.camera;
  const single = parts.length === 1 ? parts[0] : undefined;
  const canvas = (app.snapshot?.project as { canvas?: { width: number; height: number } } | undefined)?.canvas ?? { width: 1920, height: 1080 };

  const record = () => {
    const common = { mic: mic || undefined, camera: camera || undefined, resolution: settings.resolution };
    let options: TakeOptions;
    if (single === "screen") options = { mode: "screen", place: settings.place };
    else if (single === "camera") options = { mode: "camera", camera: common.camera, resolution: settings.resolution, withMic: false, place: settings.place };
    else if (single === "voiceover") options = { mode: "voiceover", mic: common.mic, playTimeline: settings.play, muteTimeline: settings.mute };
    else if (single === "system") options = { mode: "system", playTimeline: settings.play, muteTimeline: settings.mute };
    else if (!video) options = { mode: on.voiceover ? "voiceover" : "system", parts, mic: common.mic, playTimeline: settings.play, muteTimeline: settings.mute };
    else options = { mode: "multi", parts, ...common, layout: settings.layout };
    void startTake(options);
  };
  const label = single === "screen" ? "Record screen" : single === "voiceover" ? "Record voiceover" : single === "camera" ? "Record camera" : single === "system" ? "Record sound" : parts.length ? editorText('Record {0} sources',{0:parts.length}) : "Record";

  return (
    <div className="lib-section capture-browser" data-testid="capture-browser" data-phase={phase} data-parts={parts.join(",")} {...(recordedMs !== undefined ? { "data-recorded-ms": recordedMs } : {})} {...(offsets ? { "data-offsets": JSON.stringify(offsets) } : {})}>
      {/* D267: Record first, then what it records. */}
      <RecordBar label={label} meter={micOn||on.system} disabled={!canRecord} onRecord={record} />
      <CaptureRecovery/>
      {message && (
        <div className={`capture-message small ${message.kind}`} data-testid={`capture-take-${message.kind}`}>
          {editorText(message.text)}
        </div>
      )}
      <div className="capture-toggles" role="group" aria-label={editorText("What to record")} data-testid="capture-toggles">
        {SOURCES.map(({ kind, label: name, icon: Icon }) => (
          <button
            key={kind}
            className={`capture-toggle${on[kind] ? " on" : ""}`}
            data-testid={`capture-src-${kind}`}
            data-on={on[kind] ? "yes" : "no"}
            aria-pressed={on[kind]}
            disabled={busy}
            title={on[kind] ? editorText("Don't record {0}", { 0: name.toLowerCase() }) : editorText("Record {0} too", { 0: name.toLowerCase() })}
            onClick={() => set({ on: { ...settings.on, [kind]: !settings.on[kind] } })}
          >
            <Icon />
            <span>{name}</span>
            <span className="capture-toggle-dot" />
          </button>
        ))}
      </div>
      {!app.snapshot && <div className="small muted">{editorText("Open a project to record: takes go into its media folder.")}</div>}
      {on.screen && <Access kind="screen"/>}
      {(screenOn || cameraOn) && <Composite camera={cameraOn} screen={screenOn} layout={settings.layout} canvas={canvas} />}
      {on.camera && (
        <SourceCard kind="camera">
          <Access kind="camera">
            <div className="picker-row">
              <Picker icon={Video} value={camera} onChange={(v) => set({ camera: v })} testId="capture-camera-device" title={editorText("Camera")} disabled={busy}>
                {cameras.map((c) => (
                  <option key={c.id} value={c.id}>
                    {editorText(c.label)}
                  </option>
                ))}
              </Picker>
              <div className="seg capture-resolution" data-testid="capture-camera-resolution">
                {(["720p", "1080p"] as const).map((r) => (
                  <button key={r} className={settings.resolution === r ? "active" : ""} data-value={r} disabled={busy} onClick={() => set({ resolution: r })}>
                    {r}
                  </button>
                ))}
              </div>
            </div>
            {screenOn && (
              <div className="seg capture-layouts" data-testid="capture-layout" title={editorText("Where the camera sits over the screen")}>
                {MULTI_LAYOUTS.map((l) => (
                  <button key={l} className={settings.layout === l ? "active" : ""} data-value={l} title={editorText(LAYOUT_NAMES[l])} disabled={busy} onClick={() => set({ layout: l })}>
                    <span className={`layout-glyph ${l}`} />
                  </button>
                ))}
              </div>
            )}
          </Access>
        </SourceCard>
      )}
      {on.voiceover && (
        <SourceCard kind="voiceover">
          <Access kind="microphone">
            <MicSelect value={mic} onChange={(v) => set({ mic: v })} testId="capture-mic-device" disabled={busy} />
            <LevelMeter />
          </Access>
        </SourceCard>
      )}
      {on.system && (
        <SourceCard kind="system">
            <Access kind="screen">
              <div className="small muted">{editorText("Enable audio in the browser sharing dialog.")}</div>
            </Access>
        </SourceCard>
      )}
      {parts.length > 0 && (!video || single) && (
        <div className="opt-row" data-testid="capture-options">
          {!video && (
            <>
              <Option testId="capture-play-timeline" checked={settings.play} disabled={busy} onChange={(v) => set({ play: v })} title={editorText("Play the timeline from the playhead while recording")}>
                {editorText("Play timeline")}</Option>
              <Option testId="capture-mute-timeline" checked={settings.mute} disabled={busy || !settings.play} onChange={(v) => set({ mute: v })} title={editorText("Mute it, so the microphone only hears you")}>
                {editorText("Muted")}</Option>
            </>
          )}
          {single && video && (
            <Option testId="capture-place" checked={settings.place} disabled={busy} onChange={(v) => set({ place: v })} title={editorText("Also put it on the timeline at the playhead")}>
              {editorText("On the timeline")}</Option>
          )}
        </div>
      )}
    </div>
  );
}

/** One source's settings under the toggles. */
function SourceCard({ kind, children }: { kind: PartKind; children: React.ReactNode }) {
  useEditorLanguage();
  const { label, icon: Icon } = SOURCES.find((s) => s.kind === kind)!;
  return (
    <div className="capture-card" data-testid={`capture-card-${kind}`}>
      <div className="capture-card-head">
        <Icon />
        {label}
      </div>
      <div className="capture-card-body">{children}</div>
    </div>
  );
}
