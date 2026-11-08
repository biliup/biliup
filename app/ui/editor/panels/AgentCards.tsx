import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { editorRoot } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The production process in the Agent tab (U8, U9, U15, U16; D187–D189, D197): the question card
// (ask_user), the storyboard to approve, the cards a turn keeps in the conversation where they
// happened (question and answer, brief, storyboard, review, brand; the latest brief is edited and
// the latest storyboard approved on its card) and the pinned Plan chip that jumps to the latest
// brief, storyboard and review cards.
import { type AgentApproval, type AgentCard, type AgentPlan, type AgentQuestion, type Review } from "@/app/lib/editor/contracts";
import { Check, ChevronDown, ChevronRight, ClipboardList, Clapperboard, Gauge, HelpCircle, Palette, Pencil } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { actions, useApp } from "@/app/lib/editor/state/store";
import { type CardLive, BriefForm, BriefBody } from "@/app/ui/editor/panels/agent/brief";
import { BeatRow } from "@/app/ui/editor/panels/agent/storyboard";
import { ReviewBody } from "@/app/ui/editor/panels/agent/review";
import { useApproval } from "@/app/ui/editor/panels/agent/approvals";
export { StoryboardApproval } from "@/app/ui/editor/panels/agent/storyboard";
export { ScoreBars } from "@/app/ui/editor/panels/agent/review";
export type { CardLive } from "@/app/ui/editor/panels/agent/brief";

/** D187: the agent's question; pick, write, Send (or Skip). It waits up to 15 minutes. */
export function QuestionCard({ approval, question, online }: { approval: AgentApproval; question: AgentQuestion; online: boolean }) {
  useEditorLanguage();
  const [picked, setPicked] = useState<ReadonlyArray<string>>([]);
  const [text, setText] = useState("");
  const { sent, answer } = useApproval(approval, online);
  const toggle = (label: string) => setPicked((now) => (question.multi ? (now.includes(label) ? now.filter((x) => x !== label) : [...now, label]) : now[0] === label ? [] : [label]));
  const ready = picked.length > 0 || text.trim() !== "";
  const send = (allow: boolean) => {
    if (allow && !ready) return;
    void answer(allow, allow ? { answer: { choices: [...picked], ...(text.trim() ? { text: text.trim() } : {}) } } : {});
  };
  return (
    <div className="approval question-card" data-testid="agent-question" data-id={approval.id}>
      <div className="approval-text">
        <HelpCircle />
        {question.question}
      </div>
      {question.options.length > 0 && (
        <div className="question-options" role={question.multi ? "group" : "radiogroup"}>
          {question.options.map((o) => (
            <button key={o.label} className={`question-option ${picked.includes(o.label) ? "on" : ""}`} data-testid="agent-question-option" aria-pressed={picked.includes(o.label)} disabled={!online || sent} onClick={() => toggle(o.label)}>
              <span className="question-mark">{picked.includes(o.label) ? <Check /> : null}</span>
              <span className="question-label">
                {editorText(o.label)}
                {o.detail && <span className="question-detail">{editorText(o.detail)}</span>}
              </span>
            </button>
          ))}
        </div>
      )}
      {question.freeText && (
        <textarea
          className="field input question-text"
          data-testid="agent-question-text"
          rows={2}
          placeholder={question.options.length ? editorText("Or in your words") : editorText("Your answer")}
          value={text}
          disabled={!online || sent}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.metaKey || e.ctrlKey) && !e.nativeEvent.isComposing && e.keyCode !== 229 && ready) {
              e.preventDefault();
              send(true);
            }
          }}
        />
      )}
      <div className="approval-btns">
        <button className="btn ghost" data-testid="agent-question-skip" disabled={!online || sent} onClick={() => send(false)}>
          {editorText("Skip")}</button>
        <button className="btn primary" data-testid="agent-question-send" disabled={!online || sent || !ready} onClick={() => send(true)}>
          {editorText("Send")}</button>
      </div>
    </div>
  );
}

const OUTCOME: Readonly<Record<string, string>> = { approved: "Approved", declined: "Changes asked", auto: "Approved by the setting", self: "Approved by the agent", skipped: "No answer", answered: "Answered" };

/** The review's chip: passed, its blockers, or under the bar. */
const reviewOutcome = (r: Review) => (r.passed ? editorText("Passed") : r.blockers?.length ? editorText("{0} blockers", { 0: r.blockers.length }) : editorText("Below 8"));

/** The Plan chip's jump: the event a card listens for to open itself. */
const OPEN_CARD = "biliup-open-card";

/** A card a turn keeps (D186–D189), in the conversation where it happened; `live` on the latest of its type. */
export function CardItem({ card, live }: { card: AgentCard; live?: CardLive | undefined }) {
  useEditorLanguage();
  const [open, setOpen] = useState(card.type === "review" || card.type === "question");
  const [editing, setEditing] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const app = useApp();
  const intended = app.agent?.plan?.intended ?? [];
  const online = app.connection === "connected" && app.biliupCanEdit === true;
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const onOpen = () => setOpen(true);
    el.addEventListener(OPEN_CARD, onOpen);
    return () => el.removeEventListener(OPEN_CARD, onOpen);
  }, []);
  if (card.type === "question" && card.question) {
    const answer = card.answer ? [...card.answer.choices, ...(card.answer.text ? [card.answer.text] : [])].join(", ") : "";
    return (
      <div className="card-item" data-testid="agent-card" data-type="question" data-outcome={card.outcome}>
        <div className="card-head">
          <HelpCircle />
          <span className="grow">{card.question.question}</span>
        </div>
        <div className={`card-answer ${answer ? "" : "muted"}`} data-testid="agent-card-answer">
          {answer || editorText("Skipped")}
        </div>
      </div>
    );
  }
  if (card.type === "brand") {
    return (
      <div className="card-item" data-testid="agent-card" data-type="brand">
        <div className="card-head">
          <Palette />
          <span className="grow">{card.text}</span>
        </div>
      </div>
    );
  }
  // The latest brief shows the brief as it is now (the user may have edited it since).
  const brief = live && card.type === "brief" ? (live.plan.brief ?? card.brief) : card.brief;
  const sb = live && card.type === "storyboard" ? live.plan.storyboard : undefined;
  const head =
    card.type === "brief" ? (
      <>
        <ClipboardList />
        <span className="grow ellipsis">{editorText("Brief: ")}{brief?.film}</span>
        {brief?.by === "user" && live && <span className="card-outcome approved">{editorText("Edited by you")}</span>}
      </>
    ) : card.type === "storyboard" ? (
      <>
        <Clapperboard />
        <span className="grow ellipsis">{editorText("Storyboard, ")}{card.storyboard?.beats.length ?? 0} {editorText(" beats")}</span>
        {sb && sb.status === "approved" && card.outcome !== "approved" && card.outcome !== "auto" && card.outcome !== "self" ? <span className="card-outcome approved">{editorText("Approved")}</span> : card.outcome && <span className={`card-outcome ${card.outcome}`}>{editorText(OUTCOME[card.outcome] ?? card.outcome)}</span>}
      </>
    ) : (
      <>
        <Gauge />
        <span className="grow">{editorText("Review, round ")}{card.review?.round}</span>
        {card.review && <span className={`card-outcome ${card.review.passed ? "approved" : "declined"}`}>{reviewOutcome(card.review)}</span>}
      </>
    );
  return (
    <div ref={ref} className={`card-item ${open ? "open" : ""}`} data-testid="agent-card" data-type={card.type} data-outcome={card.outcome} data-latest={live ? "true" : undefined} data-passed={card.review ? String(card.review.passed) : undefined}>
      <button className="card-head" aria-expanded={open} onClick={() => setOpen(!open)}>
        {head}
        {open ? <ChevronDown /> : <ChevronRight />}
      </button>
      {open && brief && card.type === "brief" && (editing && live ? <BriefForm brief={brief} revision={live.plan.revision} onDone={() => setEditing(false)} /> : <BriefBody brief={brief} live={live} />)}
      {open && brief && card.type === "brief" && live && !editing && (
        <div className="approval-btns card-btns">
          <button className="btn ghost" data-testid="plan-brief-edit" disabled={!live.online || app.agentPlanSaving} onClick={() => setEditing(true)}>
            <Pencil />
            {editorText("Edit")}</button>
        </div>
      )}
      {open && card.storyboard && (
        <div className="beats">
          {card.storyboard.beats.map((b, i) => (
            <BeatRow key={i} beat={b} i={i} edit={false} />
          ))}
        </div>
      )}
      {open && sb && sb.status !== "approved" && !live?.waiting && (
        <div className="approval-btns card-btns">
          {sb.note && <span className="small muted grow">{editorText("You asked: ")}{sb.note}</span>}
          <button className="btn primary" data-testid="plan-storyboard-approve" disabled={!live?.online || live.busy || app.agentPlanSaving} title={live?.busy ? editorText("The agent is working") : undefined} onClick={() => { if (live) void actions.agentPlanSave({ storyboard: { ...sb, status: "approved" } }, live.plan.revision); }}>
            {editorText("Approve")}</button>
        </div>
      )}
      {open && card.review && <ReviewBody review={card.review} intended={intended} online={online} />}
    </div>
  );
}

/** Jumps to the latest card of a type in the thread: scrolled into view, opened, flashed. Whether there was one. */
export function jumpToCard(type: "brief" | "storyboard" | "review"): boolean {
  const all = editorRoot().querySelectorAll<HTMLElement>(`[data-testid="agent-card"][data-type="${type}"]`);
  const el = all[all.length - 1];
  if (!el) return false;
  el.dispatchEvent(new Event(OPEN_CARD));
  el.scrollIntoView({ block: "start", behavior: "smooth" });
  // An attribute, not a class: React owns className and the card re-renders as it opens.
  el.removeAttribute("data-flash");
  setTimeout(() => el.setAttribute("data-flash", "on"), 30);
  setTimeout(() => el.removeAttribute("data-flash"), 1600);
  return true;
}

/**
 * D197: the pinned Plan chip over the thread. The brief, storyboard and reviews are cards in the
 * conversation; the chip lists the latest of each with its state and jumps to its card (or shows
 * it here when no card has it, e.g. a brief.json written by hand).
 */
export function PlanChip({ plan }: { plan: AgentPlan }) {
  useEditorLanguage();
  const [open, setOpen] = useState(false);
  const [shown, setShown] = useState<"brief" | "storyboard" | "review" | undefined>();
  const last = plan.reviews.at(-1);
  const sb = plan.storyboard;
  if (!plan.brief && !sb && !last) return null;
  const go = (type: "brief" | "storyboard" | "review") => {
    if (jumpToCard(type)) {
      setOpen(false);
      setShown(undefined);
    } else setShown(shown === type ? undefined : type);
  };
  const state = (cls: string, text: string) => <span className={`plan-state ${cls}`}>{editorText(text)}</span>;
  return (
    <div className="plan-chip-wrap" data-testid="agent-plan">
      <button className={`plan-chip-btn ${open ? "on" : ""}`} data-testid="plan-chip" aria-expanded={open} onClick={() => (setOpen(!open), setShown(undefined))}>
        <ClipboardList />
        {editorText("Plan")}{last && state(last.passed ? "passed" : "low", reviewOutcome(last))}
        {!last && sb && state(sb.status, sb.status === "approved" ? "Approved" : sb.status === "declined" ? "Changes asked" : "Waiting")}
      </button>
      {open && (
        <div className="plan-menu" data-testid="plan-menu">
          {plan.brief && (
            <button className="plan-row" data-testid="plan-brief" onClick={() => go("brief")}>
              <ClipboardList />
              <span className="grow ellipsis">{editorText("Brief: ")}{plan.brief.film}</span>
            </button>
          )}
          {sb && (
            <button className="plan-row" data-testid="plan-storyboard" onClick={() => go("storyboard")}>
              <Clapperboard />
              <span className="grow">{editorText("Storyboard, ")}{sb.beats.length} {editorText(" beats")}</span>
              {state(sb.status, sb.status === "approved" ? "Approved" : sb.status === "declined" ? "Changes asked" : "Waiting")}
            </button>
          )}
          {last && (
            <button className="plan-row" data-testid="plan-review" onClick={() => go("review")}>
              <Gauge />
              <span className="grow">{editorText("Review, round ")}{last.round}</span>
              {state(last.passed ? "passed" : "low", reviewOutcome(last))}
            </button>
          )}
          {shown === "brief" && plan.brief && (
            <div className="plan-body">
              <BriefBody brief={plan.brief} />
            </div>
          )}
          {shown === "storyboard" && sb && (
            <div className="plan-body">
              {sb.summary && <div className="small">{sb.summary}</div>}
              <div className="beats">
                {sb.beats.map((b, i) => (
                  <BeatRow key={i} beat={b} i={i} edit={false} />
                ))}
              </div>
            </div>
          )}
          {shown === "review" && last && (
            <div className="plan-body">
              <ReviewBody review={last} intended={plan.intended ?? []} online={false} />
            </div>
          )}
        </div>
      )}
    </div>
  );
}
