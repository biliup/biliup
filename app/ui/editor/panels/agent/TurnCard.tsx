import {apiResourceUrl} from "@/app/lib/renders";
import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type AgentTurn, type AgentApproval, type AgentEdit, type AgentFile } from "@/app/lib/editor/contracts";
import { ChevronDown, ChevronRight, FolderOpen, Square, Undo2, PencilLine, Pencil, Download } from "lucide-react";
import {apiFetch,handleResponse} from '@/app/lib/api-streamer';
import { useState, useRef, useEffect, type ReactNode, Fragment } from "react";
import { withChips } from "@/app/ui/editor/panels/Composer";
import { type CardLive, CardItem, QuestionCard, StoryboardApproval } from "@/app/ui/editor/panels/AgentCards";
import { agentImageUrl, actions, useAppSelect } from "@/app/lib/editor/state/store";
import { useFriendly } from "@/app/ui/editor/panels/agent/context";
import { noteLines } from "@/app/ui/editor/panels/agent/thread";
import { groupItems, WorkLog, EditRow, MediaRow, Thinking, NowLine } from "@/app/ui/editor/panels/agent/workLog";
import { CopyButton } from "@/app/ui/editor/panels/agent/copy";
import { ScreenApprovalCard, BrandDiffCard, ApprovalCard, CleanupLine } from "@/app/ui/editor/panels/agent/approvals";

export function TurnCard({ turn: t, busy, online, approval, latest, live }: { turn: AgentTurn; busy: boolean; online: boolean; approval?: AgentApproval | undefined; latest: ReadonlySet<string>; live: CardLive | undefined }) {
  useEditorLanguage();
  const undoing = useAppSelect(state => state.agentUndoing);
  const friendly = useFriendly();
  const undone = t.undo.state === "undone";
  const status =
    // While it runs, the now line in the card says what it's doing (D139).
    t.status === "running" ? null : undone ? (
      "Undone"
    ) : t.status === "cancelled" ? (
      "Stopped"
    ) : t.status === "error" ? (
      "Didn't finish"
    ) : (
      `${t.model ?? editorText("Agent")}${t.seconds !== undefined ? `, ${Math.max(1, Math.round(t.seconds))} s` : ""}`
    );
  // D9 / D99: what generators billed to the user's keys cost in this turn (an estimate).
  const spend = t.spend ? (
    <span className="turn-spend" data-testid="agent-turn-spend" title={editorText("Estimated cost of generators billed to your API keys in this turn")}>
      {editorText("About $")}{t.spend.toFixed(2)} {editorText(" on your keys")}</span>
  ) : null;
  const canShowUndo = t.undo.state === "available" || t.undo.state === "blocked" || t.undo.state === "unavailable";
  return (
    <div className="turn-wrap" data-testid="agent-turn" data-turn={t.id} data-status={t.status} data-undo={t.undo.state}>
      {t.images && t.images.length > 0 && (
        <div className="msg-images" data-testid="agent-turn-images">
          {t.images.map((name) => {
            const src = agentImageUrl(name);
            return src ? <img key={name} src={src} alt={editorText("Attached image")} data-testid="agent-turn-image" /> : null;
          })}
        </div>
      )}
      <div className="msg-user" data-testid="agent-prompt">
        {withChips(t.prompt, (t.mentions ?? []).map((m) => m.label))}
      </div>
      {t.folders && t.folders.length > 0 && (
        <div className="msg-folders small" data-testid="agent-turn-folders">
          {t.folders.map((f) => (
            <div key={f.path} className="msg-folder" data-testid="agent-turn-folder" data-problem={f.problem ? "1" : undefined} title={f.path}>
              <FolderOpen />
              <span className="msg-folder-text">{f.problem ? editorText("Didn't list {0}: {1}", { 0: f.path, 1: f.problem }) : editorText("Found {0}{1} in {2}", { 0: f.files === 1 ? "1 file" : `${f.files} files`, 1: f.folders ? ` and ${f.folders === 1 ? "1 folder" : `${f.folders} folders`}` : "", 2: f.path })}</span>
            </div>
          ))}
        </div>
      )}
      {t.note && <ManualNote note={t.note} />}
      {/* D203: the turn as a stream of messages in order: replies, thinking, folded tool calls, edits
          where they happened, imports as one row, cards; then a compact footer. */}
      <div className={`turn ${t.status} ${undone ? "undone" : ""}`} data-testid="agent-stream">
        {groupItems(t.items, t.edits).map((g) =>
          g.kind === "steps" ? (
            <WorkLog key={g.at} steps={g.items} />
          ) : g.kind === "edit" ? (
            <EditRow key={g.at} step={g.item} edits={g.edits} />
          ) : g.kind === "media" ? (
            <MediaRow key={g.at} step={g.item} edits={g.edits} />
          ) : g.item.kind === "thinking" ? (
            <Thinking key={g.at} item={g.item} />
          ) : g.item.kind === "card" && g.item.card ? (
            <CardItem key={g.at} card={g.item.card} live={latest.has(`${t.id}:${g.at}`) ? live : undefined} />
          ) : (
            <Reply key={g.at} text={friendly(g.item.text)} live={!!g.item.live} />
          ),
        )}
        {t.files?.map(file=><SubtitleDownload key={file.id} file={file} />)}
        {approval &&
          (approval.screen ? (
            <ScreenApprovalCard key={approval.id} approval={approval} screen={approval.screen} online={online} />
          ) : approval.question ? (
            <QuestionCard key={approval.id} approval={approval} question={approval.question} online={online} />
          ) : approval.storyboard ? (
            <StoryboardApproval key={approval.id} approval={approval} storyboard={approval.storyboard} online={online} />
          ) : approval.brand ? (
            <BrandDiffCard key={approval.id} approval={approval} brand={approval.brand} online={online} />
          ) : (
            <ApprovalCard key={approval.id} approval={approval} online={online} />
          ))}
        {t.cleanup && <CleanupLine cleanup={t.cleanup} />}
        {t.status === "running" && <NowLine turn={t} waiting={!!approval} />}
        {t.error && (
          <p className="err" data-testid="agent-turn-error">
            {t.error}
          </p>
        )}
      </div>
      <div className={`turn-foot ${undone ? "undone" : ""}`}>
        {t.edits.length > 0 && <EditsFooter edits={t.edits} undone={undone} />}
        <span className="turn-status" title={status ?? undefined}>
          {status}
        </span>
        {spend}
        <span className="spacer" />
        {t.status === "running" && (
          <button className="link-btn" data-testid="agent-cancel" disabled={!online} onClick={() => void actions.agentCancel()}>
            <Square />
            {editorText("Stop")}</button>
        )}
        {canShowUndo && (
          <button
            className="link-btn"
            data-testid="agent-undo-turn"
            data-turn={t.id}
            disabled={!online || busy || !!undoing || t.undo.state !== "available"}
            title={t.undo.state === "available" ? editorText("Take back this turn's edits; later edits stay") : (t.undo.reason ?? editorText("Can't undo this turn"))}
            onClick={() => void actions.agentUndoTurn(t.id)}
          >
            <Undo2 />
            {editorText("Undo")}<span className="undo-word"> {editorText(" turn")}</span>
          </button>
        )}
      </div>
      {(t.undo.state === "blocked" || t.undo.state === "unavailable") && t.undo.reason && (
        <div className="turn-undo-reason" data-testid="agent-undo-reason">
          {t.undo.reason}
        </div>
      )}
    </div>
  );
}

/** D203: the turn's edits in the footer: "3 edits", opened for every edit and its lines. */
function EditsFooter({ edits, undone }: { edits: ReadonlyArray<AgentEdit>; undone: boolean }) {
  useEditorLanguage();
  const friendly = useFriendly();
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  // A click anywhere else closes it.
  useEffect(() => {
    if (!open) return;
    const close = (e: PointerEvent) => !box.current?.contains(e.target as Node) && setOpen(false);
    document.addEventListener("pointerdown", close, true);
    return () => document.removeEventListener("pointerdown", close, true);
  }, [open]);
  const list = (
    <div className="turn-edits-list">
      {edits.map((e) => (
        <div key={e.revision} className="turn-edit" data-testid="agent-edit">
          <div className="turn-edit-summary">{friendly(e.summary)}</div>
          {e.lines.map((line, i) => (
            <div key={i} className="turn-edit-line">
              {friendly(line)}
            </div>
          ))}
        </div>
      ))}
    </div>
  );
  return (
    <div className={`turn-edits ${open ? "open" : ""}`} data-testid="agent-edits" ref={box}>
      <button className="turn-edits-label" data-testid="agent-edits-toggle" aria-expanded={open} onClick={() => setOpen(!open)} title={open ? editorText("Hide the edits") : editorText("Show every edit")}>
        <PencilLine />
        {edits.length === 1 ? editorText("1 edit") : editorText("{0} edits", { 0: edits.length })}
        {undone ? editorText(", undone") : ""}
      </button>
      {open ? <div className="turn-edits-pop">{list}</div> : <div className="sr-only">{list}</div>}
    </div>
  );
}

/** D6: the edits by hand the agent was told about with this prompt; D295: imports folded into one line that opens. */
function ManualNote({ note }: { note: string }) {
  useEditorLanguage();
  const friendly = useFriendly();
  const [open, setOpen] = useState(false);
  const count = note.split("\n").filter((l) => l.startsWith("- ")).length;
  return (
    <div className="manual-note" data-testid="agent-note">
      <div className="manual-note-head">
        <Pencil />
        {count === 1 ? editorText("Told the agent about 1 change you made by hand") : editorText("Told the agent about {0} changes you made by hand", { 0: count })}
      </div>
      {noteLines(note).map((line, i) =>
        line.kind === "line" ? (
          <div key={i} className="manual-note-line">
            {friendly(line.text)}
          </div>
        ) : (
          <div key={i} className="manual-note-imports">
            <button className="manual-note-line manual-note-toggle" data-testid="agent-note-imports" aria-expanded={open} title={open ? editorText("Hide the files") : editorText("Show the files")} onClick={() => setOpen(!open)}>
              {open ? <ChevronDown /> : <ChevronRight />}
              {line.text}
            </button>
            {open && (
              <div className="manual-note-files" data-testid="agent-note-files">
                {line.names.map((name, j) => (
                  <div key={j} className="manual-note-line">
                    {friendly(name)}
                  </div>
                ))}
              </div>
            )}
          </div>
        ),
      )}
    </div>
  );
}

function SubtitleDownload({file}:{file:AgentFile}){
  useEditorLanguage();
  const [busy,setBusy]=useState(false),[error,setError]=useState<string>();
  const request=useRef<AbortController|null>(null);
  useEffect(()=>()=>{request.current?.abort()},[]);
  const download=async()=>{
    if(request.current)return;
    const abort=new AbortController();request.current=abort;setBusy(true);setError(undefined);
    try{
      const response=await apiFetch('/v1/editor/projects/'+file.projectId+'/agent/files/'+encodeURIComponent(file.id),{signal:abort.signal});
      await handleResponse(response);
      const blob=await response.blob();if(abort.signal.aborted)return;
      const url=URL.createObjectURL(blob),link=document.createElement('a');link.href=url;link.download=file.name;link.click();setTimeout(()=>URL.revokeObjectURL(url),60000);
    }catch(e){if(!abort.signal.aborted)setError(e instanceof Error?e.message:String(e))}
    finally{if(!abort.signal.aborted){request.current=null;setBusy(false)}}
  };
  if(file.exportJob&&Number.isSafeInteger(file.exportJob)&&file.exportJob>0)return <div className="agent-subtitle-download"><a className="btn" download href={apiResourceUrl(`/v1/editor/projects/${file.projectId}/exports/${file.exportJob}/download?attachment=true${file.format==='mp4'?'':'&subtitles=true'}`)} title={editorText(file.format==='mp4'?'Download':'Download subtitles')}><Download size={15}/><span>{file.name}</span></a></div>;
  return <div className="agent-subtitle-download"><button className="btn" type="button" onClick={()=>void download()} disabled={busy} title={editorText('Download subtitles')}><Download size={15}/><span>{busy?editorText('Downloading…'):file.name}</span></button>{error&&<p className="err" role="alert">{error}</p>}</div>;
}

/** The little markdown agents write: paragraphs, "- " bullets, **bold**, `code` and ``` code blocks (#49: each with a Copy button, like the reply). */
function Reply({ text, live = false }: { text: string; live?: boolean }) {
  useEditorLanguage();
  const inline = (s: string): ReactNode =>
    s.split(/(\*\*[^*]+\*\*|`[^`]+`)/g).map((part, i) =>
      part.startsWith("**") && part.endsWith("**") && part.length > 4 ? (
        <b key={i}>{part.slice(2, -2)}</b>
      ) : part.startsWith("`") && part.endsWith("`") && part.length > 2 ? (
        <code key={i}>{part.slice(1, -1)}</code>
      ) : (
        <Fragment key={i}>{part}</Fragment>
      ),
    );
  const prose = (chunk: string, at: number) =>
    chunk
      .split(/\n{2,}/)
      .filter((block) => block.trim() !== "")
      .map((block, i) => {
        const lines = block.split("\n");
        if (lines.every((l) => /^\s*[-*] /.test(l))) {
          return (
            <ul key={`${at}-${i}`}>
              {lines.map((l, j) => (
                <li key={j}>{inline(l.replace(/^\s*[-*] /, ""))}</li>
              ))}
            </ul>
          );
        }
        return (
          <p key={`${at}-${i}`}>
            {lines.map((l, j) => (
              <Fragment key={j}>
                {j > 0 && <br />}
                {inline(l)}
              </Fragment>
            ))}
          </p>
        );
      });
  // ``` fences (an unclosed one while it streams runs to the end).
  const parts = text.split(/^```[^\n]*\n?/m);
  return (
    <div className={`reply ${live ? "live" : ""}`} data-testid="agent-reply" data-live={live ? "true" : undefined}>
      {parts.map((part, i) =>
        i % 2 === 1 ? (
          <div key={`c${i}`} className="reply-code" data-testid="agent-code">
            <pre>{part.replace(/\n$/, "")}</pre>
            <CopyButton text={part.replace(/\n$/, "")} what="code" />
          </div>
        ) : (
          prose(part, i)
        ),
      )}
      {!live && text.trim() !== "" && <CopyButton text={text} what="reply" className="reply-copy" />}
    </div>
  );
}
