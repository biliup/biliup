import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { RefreshCw, Mic, CircleDot, Play, Pause, Square, X, type LucideIcon, ChevronDown } from "lucide-react";
import React, { useState, useEffect } from "react";
import { useRecorder, refreshAccess, levelDb, takeSeconds, type TakeMode, clock, resumeTake, pauseTake, stopTake, cancelTake, openMonitor, closeMonitor } from "@/app/ui/editor/capture/recorder";
import type { MediaAccessKind } from '../../capture/access';
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";

/** Browser permissions belong to the browser/site settings on every platform. */
export function Access({kind,children}: {kind:MediaAccessKind;children?:React.ReactNode}){
  useEditorLanguage();
  const access=useRecorder(s=>s.access?.[kind]);
  if(access===undefined)return <div className="muted small">{editorText("Checking permissions…")}</div>;
  if(access==="granted"||access==="unknown")return <>{children}</>;
  return <div className="capture-access">
    <div>{editorText(access==="restricted"?"Capture requires HTTPS or localhost and a supported browser.":"Allow this site's recording permission in browser settings.")}</div>
    <button className="btn ghost" onClick={()=>void refreshAccess()}><RefreshCw/>{editorText("Check again")}</button>
  </div>;
}

/** The meter's scale marks, dBFS. */
const SCALE = [-60, -40, -20, -12, -6, 0];
const pct = (db: number) => `${((db + 60) / 60) * 100}%`;

/**
 * D209: the microphone's level, green to amber to red, with a peak mark that holds for a moment
 * and a dB scale under it.
 */
export function LevelMeter({ testId = "capture-mic-level", compact = false }: { testId?: string; compact?: boolean }) {
  useEditorLanguage();
  const level = useRecorder((s) => s.level);
  const peak = useRecorder((s) => s.peak);
  const db = levelDb(peak);
  return (
    <div className={`level-meter ${compact ? "compact" : ""}`} title={editorText("The microphone's level")}>
      <div className="level-bar" data-testid={testId} data-level={level.toFixed(2)} data-peak={peak.toFixed(2)}>
        <div className="level-fill" style={{ clipPath: `inset(0 ${100 - level * 100}% 0 0)` }} />
        {peak > 0 && <div className={`level-peak ${peak > 0.95 ? "hot" : ""}`} data-testid="capture-level-peak" style={{ left: `calc(${peak * 100}% - 1px)` }} />}
        <span className="level-db" data-testid="capture-level-db">{Number.isFinite(db) ? editorText("{0} dB", { 0: Math.round(db) }) : editorText("-∞ dB")}</span>
      </div>
      {!compact && (
        <div className="level-scale" data-testid="capture-level-scale">
          {SCALE.map((d) => (
            <span key={d} style={{ left: pct(d) }}>
              {d}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

/** The running take's seconds, ticking. */
function useTakeSeconds() {
  const phase = useRecorder((s) => s.phase);
  const startedAt = useRecorder((s) => s.startedAt);
  const pausedMs = useRecorder((s) => s.pausedMs);
  const pausedAt = useRecorder((s) => s.pausedAt);
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (phase !== "recording") return;
    const timer = setInterval(() => setNow(Date.now()), 250);
    return () => clearInterval(timer);
  }, [phase]);
  return takeSeconds({ startedAt, pausedMs, pausedAt }, now);
}

/**
 * D209: the recording bar: a big record button; then the timer, the level meter (when the take has
 * sound), Pause, Stop and Cancel; then saving. (D238: the meter before a take is the Microphone
 * card's.) D267: the button stands alone at the top of the tab (no box around it) and a take
 * starts as soon as its devices are open, no 3-2-1 (nor the agent's take, D273).
 */
export function RecordBar({ mode, disabled = false, onRecord, label = "Record", meter = false, compact = false }: { mode?: TakeMode; disabled?: boolean; onRecord?: () => void; label?: string; meter?: boolean; compact?: boolean }) {
  useEditorLanguage();
  const phase = useRecorder((s) => s.phase);
  const count = useRecorder((s) => s.count);
  const paused = useRecorder((s) => s.paused);
  const runningMode = useRecorder((s) => s.mode);
  const seconds = useTakeSeconds();
  // D238: without a mode the bar is the one recorder's, for any take.
  const mine = mode === undefined || runningMode === mode;
  if (phase === "idle" || !mine) {
    if(!onRecord)return null;
    return (
      <button className="record-button" data-testid="capture-take-record" disabled={disabled || phase !== "idle"} onClick={onRecord}>
        <span className="record-dot" />
        <span className="record-label">{editorText(label)}</span>
      </button>
    );
  }
  if (phase === "saving") {
    return (
      <div className="record-bar" data-compact={compact || undefined} data-testid="capture-record-bar" data-state="saving">
        <span className="small muted" data-testid="capture-take-saving">
          {editorText("Saving the take…")}</span>
      </div>
    );
  }
  return (
    <div className={`record-bar live ${paused ? "paused" : ""}`} data-compact={compact || undefined} data-testid="capture-record-bar" data-state={phase === "countdown" ? "countdown" : paused ? "paused" : "recording"}>
      <div className="record-bar-row">
        {phase === "countdown" ? (
          count ? (
            <span className="capture-countdown" data-testid="capture-countdown">
              {count}
            </span>
          ) : (
            <span className="small muted" data-testid="capture-take-starting">
              {editorText("Starting…")}</span>
          )
        ) : (
          <span className={`capture-timer ${paused ? "paused" : ""}`} data-testid="capture-take-timer">
            <CircleDot />
            {clock(seconds)}
          </span>
        )}
        <div className="grow" />
        {phase === "recording" && (
          <button className="icon-btn record-ctl" data-testid={paused ? "capture-take-resume" : "capture-take-pause"} aria-label={paused ? editorText("Resume recording") : editorText("Pause recording")} title={paused ? editorText("Resume recording") : editorText("Pause recording")} onClick={() => (paused ? resumeTake() : pauseTake())}>
            {paused ? <Play /> : <Pause />}
          </button>
        )}
        {phase === "recording" && (
          <button className="record-ctl stop" data-testid="capture-take-stop" title={editorText("Stop and keep the take")} onClick={() => void stopTake()}>
            <Square />
            {editorText("Stop")}</button>
        )}
        <Tooltip label={editorText("Stop and keep nothing")}>
          <button className="icon-btn record-ctl" data-testid="capture-take-cancel" aria-label={editorText("Stop and keep nothing")} onClick={() => cancelTake()}>
            <X />
          </button>
        </Tooltip>
      </div>
      {meter && <LevelMeter testId="capture-bar-level" />}
    </div>
  );
}

/** D209: a device picker in the app's style (a native list underneath, so the keyboard and VoiceOver work). */
export function Picker({ icon: Icon, value, onChange, testId, title, disabled, children }: { icon: LucideIcon; value: string; onChange: (v: string) => void; testId: string; title: string; disabled?: boolean; children: React.ReactNode }) {
  useEditorLanguage();
  return (
    <label className={`picker ${disabled ? "disabled" : ""}`} title={title}>
      <Icon className="picker-icon" />
      <select data-testid={testId} value={value} disabled={disabled} onChange={(e) => onChange(e.target.value)}>
        {children}
      </select>
      <ChevronDown className="picker-caret" />
    </label>
  );
}

/** D209: an option as a plain checkbox and its words (a checkbox, not a chip, D263). */
export function Option({ testId, checked, disabled, onChange, children, title }: { testId: string; checked: boolean; disabled?: boolean; onChange: (on: boolean) => void; children: React.ReactNode; title?: string }) {
  useEditorLanguage();
  return (
    <label className={`opt-check ${checked ? "on" : ""} ${disabled ? "disabled" : ""}`} title={title}>
      <input type="checkbox" data-testid={testId} checked={checked} disabled={disabled} onChange={(e) => onChange(e.target.checked)} />
      <span>{children}</span>
    </label>
  );
}

export function MicSelect({ value, onChange, testId, disabled }: { value: string; onChange: (id: string) => void; testId: string; disabled?: boolean }) {
  useEditorLanguage();
  const mics = useRecorder((s) => s.mics);
  return (
    <Picker icon={Mic} value={value} onChange={onChange} testId={testId} title={editorText("Microphone")} disabled={disabled}>
      {mics.map((m) => (
        <option key={m.id} value={m.id}>
          {editorText(m.label)}
        </option>
      ))}
    </Picker>
  );
}

/** Meters the microphone while `on` (a take with sound shows the meter). */
export function useMonitor(on: boolean, mic: string) {
  useEffect(() => {
    if (!on) return;
    void openMonitor(mic || undefined);
    return () => closeMonitor(true);
  }, [on, mic]);
}

/** Shared audio availability is decided by the browser and the chosen surface. */
export function useSystemSound():{readonly can:boolean;readonly title:string}{
  const screen=useRecorder(s=>s.access?.screen);
  if(screen==="restricted")return {can:false,title:editorText("This browser does not support shared audio capture.")};
  if(screen!=="granted"&&screen!=="unknown")return {can:false,title:editorText("Allow screen sharing first.")};
  return {can:true,title:editorText("Record shared audio as a separate track")};
}
