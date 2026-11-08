import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Agent tab (D28, D48, D139-D142): turn cards with the agent's thinking, steps (a work log),
// reply and edit list, "Undo turn", the note about edits made by hand, and the composer. Live: text
// and thinking stream in, a line says what it's doing now. Names, never ids (AG5). Everything shown
// comes from the server's agent.subscribe stream; this component only sends, stops and undoes.
import { ChevronDown, ChevronRight, Maximize2, SquarePen, TriangleAlert, X } from "lucide-react";
import { useLayoutEffect, useRef, useState } from "react";

import { Composer } from "@/app/ui/editor/panels/Composer";
import { PlanChip, type CardLive } from "@/app/ui/editor/panels/AgentCards";
import { actions, useApp, type Project } from "@/app/lib/editor/state/store";
import { ProjectContext } from "@/app/ui/editor/panels/agent/context";
import { AgentEmpty, AgentTakeLine } from "@/app/ui/editor/panels/agent/status";
import { TurnCard } from "@/app/ui/editor/panels/agent/TurnCard";
import { splitThread } from "@/app/ui/editor/panels/agent/thread";
import { stageWanted, stage, useStageDismissed } from "@/app/ui/editor/workspace/agentStage";
import { panels } from "@/app/ui/editor/workspace/workspaceLayout";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";

export function AgentPanel({ project, online }: { project: Project; online: boolean }) {
  useEditorLanguage();
  const { agent, agentError, snapshot, biliupCanEdit } = useApp();
  // #75 (D255): sent to the editor while still empty: the empty state can bring the stage back.
  const dir = snapshot?.dir;
  const dismissed = useStageDismissed();
  const canRestage = biliupCanEdit === true && dir !== undefined && dismissed.includes(dir) && stageWanted(project, dir, agent, []);
  const thread = useRef<HTMLDivElement>(null);

  const writable = online && biliupCanEdit === true;
  const blocked = !biliupCanEdit || agent?.configured !== true;
  const allTurns = agent?.turns ?? [];
  const running = agent?.running ?? null;
  // D295: the current chat's turns; earlier chats fold behind a line that opens them.
  const { earlier, current: turns } = splitThread(allTurns, agent?.threadStart);
  const [showEarlier, setShowEarlier] = useState(false);
  const [loadingMore,setLoadingMore]=useState(false);
  const olderAnchor=useRef<{height:number;top:number;cursor:string}|undefined>(undefined);
  const newChat = (
    <Tooltip label={running !== null ? editorText("Wait for the agent to finish to start a new chat") : editorText("New chat: the agent starts a fresh conversation")}>
      <button className="icon-btn agent-new-chat" data-testid="agent-new-chat" aria-label={editorText("New chat")} disabled={!writable || running !== null || turns.length === 0} onClick={() => (setShowEarlier(false), void actions.agentNewThread())}>
        <SquarePen />
      </button>
    </Tooltip>
  );
  // D9: the session's estimated spend on the user's API keys (generators), when there is any.
  const sessionSpend = allTurns.reduce((n, t) => n + (t.spend ?? 0), 0);
  // D197: the latest brief and storyboard cards (edit, approve there), as "turn:item".
  const latest = new Set<string>();
  for (const type of ["brief", "storyboard"] as const) {
    let at: string | undefined;
    for (const t of allTurns) t.items.forEach((item, i) => item.kind === "card" && item.card?.type === type && (at = `${t.id}:${i}`));
    if (at) latest.add(at);
  }
  const live: CardLive | undefined = agent?.plan ? { plan: agent.plan, online:writable, busy: running !== null, waiting: !!agent.approval?.storyboard } : undefined;

  // Follow the newest turn as it streams in, but only while the thread is scrolled to its end: someone
  // reading further up keeps their place (D203), and a "Latest" button takes them down. A new turn
  // (the user just sent) always goes to the end. The empty state stays scrolled to its top.
  const stuck = useRef(true);
  const [away, setAway] = useState(false);
  const lastTurn = useRef(turns.at(-1)?.id);
  useLayoutEffect(() => {
    const el = thread.current;
    if (!el) return;
    const anchor=olderAnchor.current;
    if(anchor&&anchor.cursor!==JSON.stringify(agent?.historyCursor)){
      el.scrollTop=anchor.top+el.scrollHeight-anchor.height;
      olderAnchor.current=undefined;
      return;
    }
    const newest=turns.at(-1)?.id;
    if (newest!==lastTurn.current) stuck.current = true;
    lastTurn.current = newest;
    if (stuck.current) el.scrollTop = el.scrollHeight;
  }, [agent, turns.length]);
  const onScroll = () => {
    const el = thread.current;
    if (!el) return;
    stuck.current = el.scrollHeight - el.scrollTop - el.clientHeight < 48;
    if (away === stuck.current) setAway(!stuck.current);
  };
  const toLatest = () => {
    const el = thread.current;
    if (!el) return;
    stuck.current = true;
    setAway(false);
    el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
  };

  return (
    <ProjectContext.Provider value={project}>
    <div className="agent" data-testid="agent-panel">
      {agent?.warning && (
        <p className="err agent-warning" data-testid="agent-warning">
          <TriangleAlert />
          {agent.warning}
        </p>
      )}
      {/* An empty plan and no turns: no strip (it left a bare rule above the empty state). */}
      {(allTurns.length > 0 || (agent?.plan && (agent.plan.brief || agent.plan.storyboard || agent.plan.reviews.length > 0))) && (
        <div className="plan-strip agent-strip" data-testid="agent-strip">
          {newChat}
          <span className="spacer" />
          {agent?.plan && (agent.plan.brief || agent.plan.storyboard || agent.plan.reviews.length > 0) && <PlanChip plan={agent.plan} />}
        </div>
      )}
      <div className="agent-thread" ref={thread} data-testid="agent-thread" onScroll={onScroll}>
        {agent?.historyCursor&&<button className="link-btn agent-earlier" disabled={loadingMore||!online} onClick={async()=>{
          const el=thread.current;
          if(el)olderAnchor.current={height:el.scrollHeight,top:el.scrollTop,cursor:JSON.stringify(agent.historyCursor)};
          stuck.current=false;setShowEarlier(true);setLoadingMore(true);
          try{if(!await actions.agentMore())olderAnchor.current=undefined;}finally{setLoadingMore(false);}
        }}>{editorText(loadingMore?"Loading…":"Load earlier messages")}</button>}
        {earlier.length > 0 && (
          <button className="link-btn agent-earlier" data-testid="agent-earlier" aria-expanded={showEarlier} onClick={() => setShowEarlier(!showEarlier)}>
            {showEarlier ? <ChevronDown /> : <ChevronRight />}
            {showEarlier ? editorText("Hide earlier chat") : editorText("Earlier chat, {0}", { 0: earlier.length === 1 ? "1 turn" : `${earlier.length} turns` })}
          </button>
        )}
        {showEarlier && earlier.length > 0 && (
          <>
            {earlier.map((t) => <TurnCard key={t.id} turn={t} busy={running !== null} online={writable} approval={undefined} latest={latest} live={live} />)}
            <div className="agent-thread-divider" data-testid="agent-thread-divider">
              <span>{editorText("New chat")}</span>
            </div>
          </>
        )}
        {turns.length === 0 ? (
          <>
            <AgentEmpty name={project.name} />
            {canRestage && dir && (
              <button
                className="link-btn agent-stage-back"
                data-testid="agent-stage-reopen"
                title={editorText("Show the agent in the middle of the window")}
                onClick={() => {
                  panels.focus(false);
                  stage.reopen(dir);
                }}
              >
                <Maximize2 />
                {editorText("Open in the middle")}</button>
            )}
          </>
        ) : (
          turns.map((t) => <TurnCard key={t.id} turn={t} busy={running !== null} online={writable} approval={agent?.approval?.turnId === t.id ? agent.approval : undefined} latest={latest} live={live} />)
        )}
        <AgentTakeLine />
        {sessionSpend > 0 && (
          <div className="agent-spend small muted" data-testid="agent-session-spend">
            {editorText("About $")}{sessionSpend.toFixed(2)} {editorText(" billed to your API keys in this project so far (estimates).")}</div>
        )}
      </div>
      {away && turns.length > 0 && (
        <button className="agent-latest" data-testid="agent-latest" onClick={toLatest}>
          <ChevronDown />
          {agent?.approval ? editorText("Waiting for your answer below") : editorText("Latest")}
        </button>
      )}
      {agentError && (
        <div className="agent-error inline-error small" data-testid="agent-error">
          <span>{agentError}</span>
          <Tooltip label={editorText("Dismiss")}>
            <button className="icon-btn" onClick={actions.dismissAgentError}>
              <X />
            </button>
          </Tooltip>
        </div>
      )}
      {agent?.configured === false && <button className="btn ghost" onClick={() => actions.openSettings('app')}>{editorText('Configure model service')}</button>}
      <Composer project={project} online={online} blocked={blocked} turns={allTurns} running={running} provider={agent?.provider ?? "Model API"} />
    </div>
    </ProjectContext.Provider>
  );
}
