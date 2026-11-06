import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { StoryboardBeat, AgentApproval, Storyboard } from "@/app/lib/editor/contracts";
import { Clapperboard, Pencil } from "lucide-react";
import { useState } from "react";
import { agentImageUrl } from "@/app/lib/editor/state/store";
import { useApproval } from "./approvals";

export const seconds = (s: number) => `${Math.round(s * 10) / 10} s`;

/** One beat as a row, or its fields while editing. */
export function BeatRow({ beat, i, edit, disabled = false, onChange }: { beat: StoryboardBeat; i: number; edit: boolean; disabled?: boolean; onChange?: (b: StoryboardBeat) => void }) {
  useEditorLanguage();
  if (!edit || !onChange) {
    return (
      <div className={`beat ${beat.still ? "with-still" : ""}`} data-testid="agent-storyboard-beat">
        {beat.still && <img className="beat-still" data-testid="agent-storyboard-still" src={agentImageUrl(beat.still)} alt={editorText("Beat {0} still", { 0: i + 1 })} title={editorText("Beat {0}", { 0: i + 1 })} />}
        <div className="beat-head">
          <span className="beat-time">
            {i + 1}. {seconds(beat.start)} {editorText(" to ")}{seconds(beat.end)}
          </span>
          <b>{beat.title || editorText("Beat {0}", { 0: i + 1 })}</b>
        </div>
        <div className="beat-line">
          {beat.entry} <span className="muted">→</span> {beat.exit}
        </div>
        <div className="beat-line muted">{beat.purpose}</div>
        {(beat.assets.length > 0 || beat.sound) && (
          <div className="beat-line small muted">
            {beat.assets.join(", ")}
            {beat.assets.length > 0 && beat.sound ? ", " : ""}
            {beat.sound ? editorText("on {0}", { 0: beat.sound }) : ""}
          </div>
        )}
      </div>
    );
  }
  return <BeatFields beat={beat} disabled={disabled} onChange={onChange} />;
}

function BeatFields({ beat, disabled, onChange }: { beat: StoryboardBeat; disabled: boolean; onChange: (beat: StoryboardBeat) => void }) {
  const [assets, setAssets] = useState(beat.assets.join(", "));
  const num = (v: string, fallback: number) => (Number.isFinite(Number(v)) ? Math.max(0, Number(v)) : fallback);
  return (
    <div className="beat editing" data-testid="agent-storyboard-beat">
      <div className="beat-fields">
        <input className="field input beat-num" type="number" step="0.1" disabled={disabled} value={beat.start} title={editorText("Start, seconds")} onChange={(e) => onChange({ ...beat, start: num(e.target.value, beat.start) })} />
        <input className="field input beat-num" type="number" step="0.1" disabled={disabled} value={beat.end} title={editorText("End, seconds")} onChange={(e) => onChange({ ...beat, end: num(e.target.value, beat.end) })} />
        <input className="field input grow" data-testid="agent-storyboard-beat-title" disabled={disabled} value={beat.title} placeholder={editorText("Title")} onChange={(e) => onChange({ ...beat, title: e.target.value })} />
      </div>
      <input className="field input" disabled={disabled} value={beat.entry} placeholder={editorText("Starts on")} onChange={(e) => onChange({ ...beat, entry: e.target.value })} />
      <input className="field input" disabled={disabled} value={beat.exit} placeholder={editorText("Ends on")} onChange={(e) => onChange({ ...beat, exit: e.target.value })} />
      <input className="field input" disabled={disabled} value={beat.purpose} placeholder={editorText("Purpose")} onChange={(e) => onChange({ ...beat, purpose: e.target.value })} />
      <input className="field input" disabled={disabled} value={assets} placeholder={editorText("Assets")} onChange={(e) => {
        setAssets(e.target.value);
        onChange({ ...beat, assets: e.target.value.split(/[,，]/).map((x) => x.trim()).filter(Boolean) });
      }} />
    </div>
  );
}

/** D188: the storyboard to approve before the build: Approve (with edits) or Ask for changes. */
export function StoryboardApproval({ approval, storyboard, online }: { approval: AgentApproval; storyboard: Storyboard; online: boolean }) {
  useEditorLanguage();
  const [beats, setBeats] = useState<ReadonlyArray<StoryboardBeat>>(storyboard.beats);
  const [edit, setEdit] = useState(false);
  const [changing, setChanging] = useState(false);
  const [note, setNote] = useState("");
  const { sent, answer } = useApproval(approval, online);
  const edited = JSON.stringify(beats) !== JSON.stringify(storyboard.beats);
  const approve = () => {
    void answer(true, edited ? { beats } : {});
  };
  const change = () => {
    if (note.trim()) void answer(false, { note: note.trim() });
  };
  return (
    <div className="approval storyboard-card" data-testid="agent-storyboard" data-id={approval.id}>
      <div className="approval-text">
        <Clapperboard />
        {storyboard.summary || editorText("Storyboard")}
      </div>
      <div className="beats">
        {beats.map((b, i) => (
          <BeatRow key={i} beat={b} i={i} edit={edit} disabled={!online || sent} onChange={(next) => setBeats(beats.map((x, j) => (j === i ? next : x)))} />
        ))}
      </div>
      {changing && <textarea className="field input" data-testid="agent-storyboard-note" rows={2} placeholder={editorText("What to change")} value={note} disabled={!online || sent} autoFocus onChange={(e) => setNote(e.target.value)} />}
      <div className="approval-sub">{editorText("The build waits for your approval.")}</div>
      <div className="approval-btns">
        {!changing && (
          <button className="btn ghost" data-testid="agent-storyboard-edit" disabled={!online || sent} onClick={() => setEdit(!edit)}>
            <Pencil />
            {edit ? editorText("Done editing") : editorText("Edit")}
          </button>
        )}
        {changing ? (
          <button className="btn" data-testid="agent-storyboard-send-change" disabled={!online || sent || !note.trim()} onClick={change}>
            {editorText("Send changes")}</button>
        ) : (
          <button className="btn ghost" data-testid="agent-storyboard-change" disabled={!online || sent} onClick={() => setChanging(true)}>
            {editorText("Ask for changes")}</button>
        )}
        <button className="btn primary" data-testid="agent-storyboard-approve" disabled={!online || sent} onClick={approve}>
          {edited ? editorText("Approve with edits") : editorText("Approve")}
        </button>
      </div>
    </div>
  );
}
