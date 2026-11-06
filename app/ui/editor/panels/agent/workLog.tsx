import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { AgentItem, AgentEdit, AgentTurn } from "@/app/lib/editor/contracts";
import { LoaderCircle, X, PencilLine, ChevronDown, ChevronRight, Images, Music, FileImage, Film, Check, Brain } from "lucide-react";
import { useState, useContext, useEffect, useRef } from "react";
import { thumbUrl } from "@/app/lib/editor/state/store";
import type { MediaItem } from '@/app/lib/editor/state/project';
import { usePoster, usePreviewVisible } from '@/app/ui/editor/preview/resources';
import { useFriendly, ProjectContext } from "@/app/ui/editor/panels/agent/context";
import { CopyButton, selecting } from "@/app/ui/editor/panels/agent/copy";

type Group =
  | { kind: "steps"; at: number; items: Array<AgentItem> }
  | { kind: "edit" | "media"; at: number; item: AgentItem; edits: Array<AgentEdit> }
  | { kind: "one"; at: number; item: AgentItem };

/**
 * Consecutive tool calls make one work log; text, thinking and cards stand alone. D203: a call that
 * edited the project is its own row, where it happened (its edit's `at`); one that added media
 * (an import) is one row with the files as thumbnails.
 */
export function groupItems(items: ReadonlyArray<AgentItem>, edits: ReadonlyArray<AgentEdit> = []): Array<Group> {
  const byStep = new Map<number, Array<AgentEdit>>();
  for (const e of edits) if (e.at !== undefined) byStep.set(e.at, [...(byStep.get(e.at) ?? []), e]);
  const out: Array<Group> = [];
  items.forEach((item, at) => {
    // Thinking with no words (yet) isn't a row (D182).
    if (item.kind === "thinking" && !item.text.trim()) return;
    const made = item.kind === "step" ? byStep.get(at) : undefined;
    if (made) return void out.push({ kind: made.some((e) => e.media?.length) ? "media" : "edit", at, item, edits: made });
    const last = out.at(-1);
    if (item.kind === "step" && last?.kind === "steps") last.items.push(item);
    else if (item.kind === "step") out.push({ kind: "steps", at, items: [item] });
    else out.push({ kind: "one", at, item });
  });
  return out;
}

/** D203: a call that edited the project: its summary, opened for what it changed. */
export function EditRow({ step, edits }: { step: AgentItem; edits: ReadonlyArray<AgentEdit> }) {
  useEditorLanguage();
  const friendly = useFriendly();
  const [open, setOpen] = useState(false);
  const lines = edits.flatMap((e) => e.lines);
  return (
    <div className={`edit-row ${open ? "open" : ""}`} data-testid="agent-edit-row">
      <div className="step edit-step" data-testid="agent-step" data-status={step.status} data-tool={step.tool} aria-expanded={open} title={open ? editorText("Hide the changes") : editorText("Show the changes")} onClick={() => selecting() || setOpen(!open)}>
        {step.status === "running" ? <LoaderCircle className="spin" /> : step.status === "error" ? <X className="bad" /> : <PencilLine />}
        <span className="grow">{friendly(step.status === "running" ? (step.doing ?? step.text) : step.text)}</span>
        {lines.length > 0 && (open ? <ChevronDown className="step-chev" /> : <ChevronRight className="step-chev" />)}
      </div>
      {open && (
        <div className="edit-lines" data-testid="agent-edit-lines">
          {lines.map((line, i) => (
            <div key={i}>{friendly(line)}</div>
          ))}
        </div>
      )}
    </div>
  );
}

/** D203: an import (or a generated or recorded file) as one row ("Imported 40 files"), opened to its files as thumbnails. */
export function MediaRow({ step, edits }: { step: AgentItem; edits: ReadonlyArray<AgentEdit> }) {
  useEditorLanguage();
  const project = useContext(ProjectContext);
  const [open, setOpen] = useState(false);
  const ids = edits.flatMap((e) => e.media ?? []);
  const summary = edits.length === 1 ? edits[0]!.summary : `Added ${ids.length} files`;
  return (
    <div className={`media-row ${open ? "open" : ""}`} data-testid="agent-import" data-count={ids.length}>
      <div className="step media-step" data-testid="agent-step" data-status={step.status} data-tool={step.tool} aria-expanded={open} title={open ? editorText("Hide the files") : editorText("Show the files")} onClick={() => setOpen(!open)}>
        {step.status === "running" ? <LoaderCircle className="spin" /> : <Images />}
        <span className="grow">{summary}</span>
        {open ? <ChevronDown className="step-chev" /> : <ChevronRight className="step-chev" />}
      </div>
      {open && (
        <div className="media-grid" data-testid="agent-import-grid">
          {ids.map((id) => {
            const m = project?.media.find((x) => x.id === id);
            return <ImportedPicture key={id} media={m} />;
          })}
        </div>
      )}
    </div>
  );
}

const StepIcon = ({ status }: { status?: string | undefined }) => (status === "error" ? <X className="bad" /> : status === "running" ? <LoaderCircle className="spin" /> : <Check />);

function ImportedPicture({media}: {media: MediaItem | undefined}) {
  useEditorLanguage();
  const ref = useRef<HTMLDivElement>(null);
  const visible = usePreviewVisible(ref);
  const poster = usePoster(media && media.kind !== 'audio' ? thumbUrl(media.id, media.path) : undefined, visible);
  const Icon = media?.kind === 'audio' ? Music : media?.kind === 'image' ? FileImage : Film;
  return <div ref={ref} className="media-tile" data-testid="agent-import-thumb" title={media?.name ?? editorText('No longer in the project')}>
    {poster.value ? <img src={poster.value.src} alt={media?.name ?? ''} /> : <Icon />}
    <span className="media-tile-name ellipsis">{media?.name ?? editorText('Removed')}</span>
  </div>;
}

/** Tool calls in a row (D139): one line ("Added title, and 3 more"), opened for every step. */
export function WorkLog({ steps }: { steps: ReadonlyArray<AgentItem> }) {
  useEditorLanguage();
  const friendly = useFriendly();
  const [open, setOpen] = useState(false);
  const one = (s: AgentItem, i: number) => <Step key={i} step={s} />;
  // A failed step opens the log, so its ✕ and why are in view (D182).
  const failedNow = steps.some((s) => s.status === "error");
  useEffect(() => {
    if (failedNow) setOpen(true);
  }, [failedNow]);
  if (steps.length === 1) return <div className="worklog">{one(steps[0]!, 0)}</div>;
  const failed = steps.filter((s) => s.status === "error").length;
  const runningNow = steps.some((s) => s.status === "running");
  const lead = steps.at(-1)!;
  return (
    <div className={`worklog ${open ? "open" : ""}`} data-testid="agent-worklog" data-count={steps.length}>
      <button className="worklog-head" data-testid="agent-worklog-toggle" aria-expanded={open} onClick={() => setOpen(!open)}>
        {open ? <ChevronDown /> : <ChevronRight />}
        <StepIcon status={runningNow ? "running" : failed === steps.length || lead.status === "error" ? "error" : "done"} />
        <span className="ellipsis grow">
          {friendly(lead.status === "running" ? (lead.doing ?? lead.text) : lead.text)}{editorText(", and ")}{steps.length - 1} {editorText(" more")}{failed ? <span className="worklog-failed">, {failed} {editorText(" didn't work")}</span> : null}
        </span>
      </button>
      {open && <div className="worklog-steps">{steps.map(one)}</div>}
      {/* Kept for tests and screen readers: every step, even folded. */}
      {!open && <div className="sr-only">{steps.map(one)}</div>}
    </div>
  );
}

/** One step of the work log; a failed one shows ✕, and a click shows what the tool answered (D182). */
function Step({ step: s }: { step: AgentItem }) {
  useEditorLanguage();
  const friendly = useFriendly();
  const [why, setWhy] = useState(true);
  // D194: a built-in tool's answer (search results, a page, a command's output) opens from its row.
  const [shown, setShown] = useState(false);
  const failed = s.status === "error" && !!s.error;
  const result = !failed && s.status !== "running" && !!s.result;
  // #49: a click that ends a drag-select of the row's text doesn't fold it.
  const toggle = failed ? () => selecting() || setWhy(!why) : result ? () => selecting() || setShown(!shown) : undefined;
  return (
    <>
      <div
        className={`step ${failed ? "failed" : ""} ${result ? "has-result" : ""}`}
        data-testid="agent-step"
        data-status={s.status}
        data-tool={s.tool}
        title={failed ? (why ? editorText("Hide why") : editorText("Show why")) : result ? (shown ? editorText("Hide the result") : editorText("Show the result")) : undefined}
        onClick={toggle}
        aria-expanded={result ? shown : undefined}
      >
        <StepIcon status={s.status} />
        <span className="grow">{friendly(s.status === "running" ? (s.doing ?? s.text) : s.text)}</span>
        {result && (shown ? <ChevronDown className="step-chev" /> : <ChevronRight className="step-chev" />)}
      </div>
      {failed && why && (
        <div className="step-error" data-testid="agent-step-error">
          {s.error}
        </div>
      )}
      {result && shown && (
        <div className="step-result-wrap">
          <pre className="step-result" data-testid="agent-step-result">
            {s.result}
          </pre>
          <CopyButton text={s.result ?? ""} what="output" />
        </div>
      )}
    </>
  );
}

/** A thinking summary (D139): folded to one line, opened for all of it; the latest line while it streams. Empty thinking is never a row (D182): the now line says Thinking. */
export function Thinking({ item }: { item: AgentItem }) {
  useEditorLanguage();
  const [open, setOpen] = useState(false);
  const lines = item.text.trim().split("\n").filter(Boolean);
  if (!lines.length) return null;
  return (
    <div className={`thinking ${open ? "open" : ""}`} data-testid="agent-thinking" data-live={item.live ? "true" : undefined}>
      <button className="thinking-head" onClick={() => setOpen(!open)} aria-expanded={open}>
        <Brain />
        <span className="ellipsis grow">{item.live ? (lines.at(-1) ?? editorText("Thinking")) : open ? editorText("Thinking") : (lines[0] ?? editorText("Thinking"))}</span>
        {open ? <ChevronDown /> : <ChevronRight />}
      </button>
      {open && <div className="thinking-body">{item.text}</div>}
    </div>
  );
}

/** What the agent is doing now (D139): the running step, writing, thinking, or starting. */
export function NowLine({ turn, waiting }: { turn: AgentTurn; waiting: boolean }) {
  useEditorLanguage();
  const friendly = useFriendly();
  const last = turn.items.at(-1);
  const step = [...turn.items].reverse().find((i) => i.kind === "step" && i.status === "running");
  const what = waiting
    ? editorText("Waiting for your answer")
    : step
      ? friendly(step.doing ?? step.text)
      : last?.kind === "thinking" && (last.live || !last.text.trim())
        ? editorText("Thinking")
        : last?.kind === "text" && last.live
          ? editorText("Writing reply")
          : turn.items.length === 0
            ? editorText("Starting assistant")
            : editorText("Working");
  return (
    <div className="now-line" data-testid="agent-now">
      <LoaderCircle className="spin" />
      <span className="ellipsis">{what}…</span>
    </div>
  );
}
