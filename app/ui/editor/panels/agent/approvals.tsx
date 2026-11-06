import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { AgentApproval, AgentTurn } from "@/app/lib/editor/contracts";
import { useContext, useState, useEffect, useRef } from "react";
import { actions } from "@/app/lib/editor/state/store";
import { currentEditorStore, EditorStoreContext } from "@/app/lib/editor/state/store/core";
import { useRecorder, getRecorder, startTake } from "@/app/ui/editor/capture/recorder";

/** Cards are keyed by approval ID; failed submissions retain the user's draft. */
export function useApproval(approval: AgentApproval, online: boolean) {
  const owner = useContext(EditorStoreContext);
  const [sent, setSent] = useState(false);
  const pending = useRef(false);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);
  const current = () => mounted.current && owner && currentEditorStore() === owner
    && owner.getSnapshot().agent?.approval?.id === approval.id;
  const run = async (work: () => Promise<boolean>) => {
    const state = owner?.getSnapshot();
    if (!current() || !online || pending.current || state?.connection !== "connected" || !state.biliupCanEdit) return false;
    pending.current = true;
    setSent(true);
    let accepted = false;
    try {
      accepted = await work();
      return accepted;
    } catch (error) {
      if (current()) owner?.set({ agentError: editorText("Couldn't submit the response: {0}", { 0: error instanceof Error ? error.message : String(error) }) });
      return false;
    } finally {
      if (!accepted) {
        pending.current = false;
        if (current()) setSent(false);
      }
    }
  };
  const answer = (allow: boolean, extra: Parameters<typeof actions.agentApprove>[2] = {}) =>
    run(() => actions.agentApprove(approval.id, allow, extra));
  return { sent, answer, run };
}

/** D198: the agent wants to change a kit's colours or fonts: each field before and after; Apply or Keep the kit (no answer in 2 minutes keeps it). */
export function BrandDiffCard({ approval, brand, online }: { approval: AgentApproval; brand: NonNullable<AgentApproval["brand"]>; online: boolean }) {
  useEditorLanguage();
  const { sent, answer } = useApproval(approval, online);
  const hex = (line: string) => /^#[0-9a-f]{6}\b/i.exec(line)?.[0];
  const row = (line: string, kind: "gone" | "new" | "same", i: number) => (
    <div key={i} className={`approval-diff-row ${kind}`} data-testid={`agent-brand-diff-${kind}`}>
      {hex(line) && <span className="approval-diff-swatch" style={{ background: hex(line) }} />}
      <span className="ellipsis">{line}</span>
    </div>
  );
  return (
    <div className="approval" data-testid="agent-approval" data-kind="brand" data-id={approval.id}>
      <div className="approval-text">{approval.text}?</div>
      <div className="approval-diff" data-testid="agent-brand-diff">
        {brand.changes.map((c) => {
          const after = new Set(c.after.map((x) => x.toLowerCase()));
          const before = new Set(c.before.map((x) => x.toLowerCase()));
          return (
            <div key={c.field}>
              <div className="approval-diff-field">{editorText(c.field)}</div>
              <div className="approval-diff-cols">
                <div>
                  <span className="approval-diff-label">{editorText("Now")}</span>
                  {c.before.map((l, i) => row(l, after.has(l.toLowerCase()) ? "same" : "gone", i))}
                </div>
                <div>
                  <span className="approval-diff-label">{editorText("After")}</span>
                  {c.after.map((l, i) => row(l, before.has(l.toLowerCase()) ? "same" : "new", i))}
                </div>
              </div>
            </div>
          );
        })}
      </div>
      <div className="approval-sub">{editorText("The kit \"")}{brand.kit}{editorText("\" keeps its colours and fonts unless you apply this. No answer in 2 minutes keeps them.")}</div>
      <div className="approval-btns">
        <button className="btn ghost" data-testid="agent-decline" disabled={!online || sent} onClick={() => answer(false)}>
          {editorText("Keep the kit")}</button>
        <button className="btn primary" data-testid="agent-allow" disabled={!online || sent} onClick={() => answer(true)}>
          {editorText("Apply")}</button>
      </div>
    </div>
  );
}

/** Empty-track cleanup is part of the turn and uses the same Undo turn action. */
export function CleanupLine({ cleanup }: { cleanup: NonNullable<AgentTurn["cleanup"]> }) {
  useEditorLanguage();
  if (!cleanup.tracks.length) return null;
  return (
    <div className="turn-cleanup" data-testid="agent-cleanup">
      {editorText("Removed {0} empty tracks", { 0: cleanup.tracks.length })}
    </div>
  );
}
/** D99: the agent wants to spend over $1 of the user's key; it waits for Allow or Don't (no answer in 2 minutes is no). */
export function ApprovalCard({ approval, online }: { approval: AgentApproval; online: boolean }) {
  useEditorLanguage();
  const { sent, answer } = useApproval(approval, online);
  return (
    <div className="approval" data-testid="agent-approval" data-kind={approval.edit ? "edit" : "spend"} data-id={approval.id}>
      <div className="approval-text">{approval.text}?</div>
      {approval.edit && approval.edit.lines.length > 0 && (
        <div className="approval-lines" data-testid="agent-approval-lines">
          {approval.edit.lines.map((line, i) => (
            <div key={i}>{line}</div>
          ))}
        </div>
      )}
      <div className="approval-btns">
        <button className="btn ghost" data-testid="agent-decline" disabled={!online || sent} onClick={() => answer(false)}>
          {editorText("Don't")}</button>
        <button className="btn primary" data-testid="agent-allow" disabled={!online || sent} onClick={() => answer(true)}>
          {editorText("Allow")}</button>
      </div>
    </div>
  );
}

/**
 * D144: recording is explicitly chosen in the browser picker, then confirmed before recording.
 * The take uses the existing recorder and is imported as an edit of the assistant turn.
 */
export function ScreenApprovalCard({ approval, screen, online }: { approval: AgentApproval; screen: NonNullable<AgentApproval["screen"]>; online: boolean }) {
  useEditorLanguage();
  const { sent, answer, run } = useApproval(approval, online);
  const [withMic,setWithMic]=useState(screen.withMic);
  const phase=useRecorder(state=>state.phase);
  const record=()=>{
    if(!online||sent||getRecorder().phase!=="idle")return;
    // Open the browser picker directly from this click. Confirmation happens before MediaRecorder starts.
    void run(async () => {
      await startTake({mode:"screen",withMic,seconds:screen.seconds,agent:{id:approval.id}});
      return getRecorder().agentId === approval.id;
    });
  };
  return <div className="approval" data-testid="agent-approval" data-kind="screen" data-id={approval.id}>
    <div className="approval-text">{approval.text}</div>
    {screen.withMic&&<label className="field"><input type="checkbox" checked={withMic} disabled={!online||sent} onChange={event=>setWithMic(event.target.checked)}/>{editorText("Microphone")}</label>}
    <div className="approval-btns">
      <button className="btn ghost" disabled={!online||sent} onClick={()=>void answer(false)}>{editorText("Don't")}</button>
      <button className="btn primary" disabled={!online||sent||phase!=="idle"} onClick={record}>{editorText("Record")}</button>
    </div>
  </div>;
}
