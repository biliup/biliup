import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { useState, useEffect } from "react";
import { useRecorder, clock, stopTake, cancelTake } from "@/app/ui/editor/capture/recorder";

// The first thing a new project shows (t3code-style): one question naming the project, no
// prefilled prompts. The composer below is where the answer goes.
export function AgentEmpty({ name }: { name: string }) {
  useEditorLanguage();
  return (
    <div className="agent-empty" data-testid="agent-empty">
      <h2 className="agent-empty-title">
        {editorText("What should we make in ")}<span className="agent-empty-name">{name || editorText("this project")}</span>?
      </h2>
    </div>
  );
}

export function AgentTakeLine() {
  useEditorLanguage();
  const agentId = useRecorder((s) => s.agentId);
  const phase = useRecorder((s) => s.phase);
  const count = useRecorder((s) => s.count);
  const startedAt = useRecorder((s) => s.startedAt);
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (phase !== "recording") return;
    const timer = setInterval(() => setNow(Date.now()), 250);
    return () => clearInterval(timer);
  }, [phase]);
  if (!agentId || phase === "idle") return null;
  return (
    <div className="agent-take small" data-testid="agent-screen-take" data-phase={phase}>
      <span className="capture-timer">{phase === "countdown" ? (count ? editorText("Recording in {0}", { 0: count }) : editorText("Starting…")) : phase === "recording" ? editorText("Recording {0}", { 0: clock((now - (startedAt ?? now)) / 1000) }) : editorText("Sending the recording…")}</span>
      {phase === "recording" && (
        <button className="link-btn" data-testid="agent-screen-stop" onClick={() => void stopTake()}>
          {editorText("Stop now")}</button>
      )}
      {phase !== "saving" && (
        <button className="link-btn" data-testid="agent-screen-cancel" onClick={() => cancelTake()}>
          {editorText("Cancel")}</button>
      )}
    </div>
  );
}
