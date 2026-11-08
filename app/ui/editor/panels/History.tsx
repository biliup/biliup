import { editorText, getEditorLanguage, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The History tab: the project's saved revisions, newest first: who changed what and
// when. A turn's edits are one group with its prompt and "Undo turn". Clicking an entry previews that
// version in the viewer (the timeline keeps showing now); Restore brings it back as a new revision,
// so one ⌘Z takes the restore back. Everything comes from history.subscribe and agent.subscribe.
import type { AgentTurn, HistoryEntry, ProjectSnapshot } from "@/app/lib/editor/contracts";
import { Download, Eye, FileText, FileWarning, History as HistoryIcon, RotateCcw, Sparkles, TriangleAlert, Undo2, User } from "lucide-react";
import { useContext, useEffect, useMemo, useRef, useState } from "react";

import { getPlayback, playback, type PreviewDraft } from "@/app/ui/editor/preview/playback";
import { actions, projectOf, useApp } from "@/app/lib/editor/state/store";
import { currentEditorStore, EditorStoreContext } from '@/app/lib/editor/state/store/core';
import {captureEditorFocus} from '../preview/focus';

type Group = { readonly key: string; readonly entries: ReadonlyArray<HistoryEntry>; readonly turnId?: string };

/** Consecutive entries of one agent turn are one group; everything else stands alone. */
export function groupEntries(entries: ReadonlyArray<HistoryEntry>): Array<Group> {
  const groups: Array<{ key: string; entries: Array<HistoryEntry>; turnId?: string }> = [];
  for (const e of entries) {
    const last = groups.at(-1);
    const turnId=e.author==='agent'&&e.kind==='edit'?e.turnId:undefined;
    if (turnId && last?.turnId === turnId) last.entries.push(e);
    else groups.push({ key: String(e.revision), entries: [e], ...(turnId ? { turnId } : {}) });
  }
  return groups;
}

const WHO: Readonly<Record<HistoryEntry["author"], string>> = { user: "You", agent: "Agent", external: "Other edits", import: "Imported" };
const who = (e: HistoryEntry) => editorText(e.kind === "initial" ? "Created" : WHO[e.author]);

/** "14:03" today, "Oct 11, 14:03" before. */
export function when(at: string, now = new Date()) {
  const d = new Date(at);
  if (Number.isNaN(d.getTime())) return "";
  const locale = getEditorLanguage();
  const time = d.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit" });
  return d.toDateString() === now.toDateString() ? time : `${d.toLocaleDateString(locale, { month: "short", day: "numeric" })}, ${time}`;
}

function AuthorIcon({ entry }: { entry: HistoryEntry }) {
  useEditorLanguage();
  if (entry.kind === "external") return <FileWarning />;
  if (entry.kind === "jump") return <RotateCcw />;
  if (entry.kind === "initial" && entry.author === "import") return <FileText />;
  if (entry.author === "agent") return <Sparkles />;
  if (entry.author === "import") return <Download />;
  return <User />;
}

export function HistoryPanel({ snapshot, online }: { snapshot: ProjectSnapshot; online: boolean }) {
  useEditorLanguage();
  const { history, agent, connection, biliupCanEdit, agentUndoing } = useApp();
  const owner = useContext(EditorStoreContext)!;
  const readable = connection === 'connected';
  const editable = online && !!biliupCanEdit;
  const [selected, setSelected] = useState<number | undefined>();
  const [preview, setPreview] = useState<{ readonly revision: number; readonly draft: PreviewDraft } | undefined>();
  const [problem, setProblem] = useState<string | undefined>();
  const [loadingMore,setLoadingMore]=useState(false);
  const [loadingPreview, setLoadingPreview] = useState<number>();
  const [restoring, setRestoring] = useState(false);
  const ownedPreview = useRef<PreviewDraft | undefined>(undefined);
  const moreRequest = useRef<object | undefined>(undefined);
  const restoreRequest = useRef<object | undefined>(undefined);
  const project = projectOf(snapshot);
  const mine = history && history.dir === snapshot.dir ? history : undefined;
  const entries = mine?.entries ?? [];
  const groups = useMemo(() => groupEntries(entries), [entries]);
  const turns = useMemo(() => new Map((agent?.turns ?? []).map((t) => [t.id, t])), [agent]);
  // The entry that is "now": the newest one whose change is in the project as it is.
  const now = entries.find((e) => e.current)?.revision;
  // The preview asked for last, and the project it was asked against: an older answer is dropped.
  const wanted = useRef<{ readonly revision: number; readonly base: unknown } | undefined>(undefined);
  const current = () => {
    const state = owner.getSnapshot();
    return currentEditorStore() === owner && state.connection === 'connected'
      && state.snapshot?.dir === snapshot.dir && state.snapshot.revision === snapshot.revision;
  };
  const dropPreview = () => {
    const draft = ownedPreview.current;
    ownedPreview.current = undefined;
    if (draft && getPlayback().draft === draft) playback.setDraft(undefined);
  };

  // A new snapshot (an edit, an undo, the restore itself) ends the preview: it showed another base.
  useEffect(() => {
    wanted.current = undefined;
    setSelected(undefined);
    setPreview(undefined);
    setProblem(undefined);
    setLoadingMore(false);
    setLoadingPreview(undefined);
    setRestoring(false);
    const release = playback.subscribe(() => {
      const draft = getPlayback().draft;
      if ((ownedPreview.current && draft !== ownedPreview.current) || (wanted.current && !ownedPreview.current && draft)) {
        ownedPreview.current = undefined;
        wanted.current = undefined;
        setSelected(undefined); setPreview(undefined); setLoadingPreview(undefined); setProblem(undefined);
      }
    });
    return () => {
      release();
      wanted.current = undefined;
      moreRequest.current = undefined;
      restoreRequest.current = undefined;
      dropPreview();
    };
  }, [owner, snapshot.revision, snapshot.dir, readable]);

  const pick = async (revision: number) => {
    if (!current() || restoreRequest.current) return;
    if (selected === revision) return close();
    dropPreview();
    setPreview(undefined);
    setSelected(revision);
    setProblem(undefined);
    if (revision === now) {
      wanted.current = undefined;
      return setLoadingPreview(undefined);
    }
    if (getPlayback().draft) return setProblem(editorText('Finish the current edit before previewing a version.'));
    playback.pause();
    const unchanged = captureEditorFocus();
    const ask = { revision, base: project };
    wanted.current = ask;
    setLoadingPreview(revision);
    const result = await actions.historyPreview(revision);
    if (wanted.current !== ask || !current()) return;
    if (!unchanged()) return close();
    setLoadingPreview(undefined);
    if ("problem" in result) return setProblem(result.problem);
    if (getPlayback().draft) return setProblem(editorText('Finish the current edit before previewing a version.'));
    const draft: PreviewDraft = { base: project, project: result.document.project, sources: result.document.sources, kind: 'history' };
    ownedPreview.current = draft;
    playback.setDraft(draft);
    setPreview({ revision, draft });
  };
  const close = () => {
    wanted.current = undefined;
    dropPreview();
    setSelected(undefined);
    setPreview(undefined);
    setLoadingPreview(undefined);
    setProblem(undefined);
  };
  const restore = async (revision: number) => {
    if (!current() || !editable || agentUndoing || restoreRequest.current || loadingPreview !== undefined) return;
    const request = {};
    restoreRequest.current = request;
    setRestoring(true); setProblem(undefined);
    try {
      const error = await actions.historyJump(revision, { dir: snapshot.dir, baseRevision: snapshot.revision });
      if (restoreRequest.current !== request || !current()) return;
      if (error) setProblem(error.message); else close();
    } finally {
      if (restoreRequest.current === request) { restoreRequest.current = undefined; setRestoring(false); }
    }
  };

  return (
    <div className="history" data-testid="history-panel">
      <div className="history-head">
        <HistoryIcon />
        <span className="grow">{editorText("History")}</span>
        {mine && (
          <span className="muted small" data-testid="history-count">
            {editorText(mine.total === 1 ? '1 change' : '{0} changes', { 0: mine.total })}
          </span>
        )}
      </div>
      {mine?.problem && (
        <div className="history-problem" data-testid="history-problem">
          <TriangleAlert />
          <span>{mine.problem}</span>
        </div>
      )}
      {preview && (
        <div className="history-preview-note" data-testid="history-previewing">
          <Eye />
          <span className="grow">{editorText('Previewing revision {0}', { 0: preview.revision })}</span>
          <button className="link-btn" onClick={close}>{editorText('Back to current version')}</button>
        </div>
      )}
      <div className="history-list" data-testid="history-list">
        {!mine && <div className="muted small history-empty">{editorText("Loading…")}</div>}
        {groups.map((g) =>
          g.turnId ? (
            <TurnGroup key={g.key} group={g} turn={turns.get(g.turnId)} now={now} selected={selected} online={editable} readable={readable} busy={restoring || !!agentUndoing} loading={loadingPreview} onPick={pick} onRestore={restore} />
          ) : (
            <Entry key={g.key} entry={g.entries[0]!} now={now} selected={selected} online={editable} readable={readable} busy={restoring || !!agentUndoing} loading={loadingPreview} onPick={pick} onRestore={restore} />
          ),
        )}
        {problem && <div className="inline-error small">{problem}</div>}
        {mine?.before!=null && <button className="link-btn history-empty" disabled={loadingMore||!readable} onClick={async()=>{
          if (!current() || moreRequest.current) return;
          const request = {}; moreRequest.current = request;
          setLoadingMore(true);setProblem(undefined);
          try { const error = await actions.historyMore(); if (moreRequest.current === request && current()) setProblem(error); }
          finally { if (moreRequest.current === request) { moreRequest.current = undefined; setLoadingMore(false); } }
        }}>{editorText(loadingMore?"Loading…":mine.problem?"Retry":"Load earlier changes")}</button>}
      </div>
    </div>
  );
}

interface RowProps {
  readonly now: number | undefined;
  readonly selected: number | undefined;
  readonly online: boolean;
  readonly readable: boolean;
  readonly busy: boolean;
  readonly loading: number | undefined;
  readonly onPick: (revision: number) => void;
  readonly onRestore: (revision: number) => void;
}

function Entry({ entry, now, selected, online, readable, busy, loading, onPick, onRestore, nested = false }: RowProps & { entry: HistoryEntry; nested?: boolean }) {
  useEditorLanguage();
  const open = selected === entry.revision;
  const isNow = entry.revision === now;
  return (
    <div
      className={`history-entry ${entry.current ? "" : "out"} ${entry.kind === "external" ? "external" : ""} ${open ? "open" : ""} ${nested ? "nested" : ""}`}
      data-testid="history-entry"
      data-revision={entry.revision}
      data-kind={entry.kind}
      data-author={entry.author}
      data-current={entry.current}
    >
      <button
        className="history-row"
        aria-expanded={open}
        disabled={!readable || busy}
        onClick={() => onPick(entry.revision)}
        title={entry.current ? editorText("Preview this version") : editorText("Not in the project now (undone or gone back past). Preview it")}
      >
        {!nested && <AuthorIcon entry={entry} />}
        <span className="history-label">
          {!nested && <span className="history-who">{who(entry)}</span>}
          <span className="history-what">{editorText(entry.label)}</span>
          {!entry.current && (
            <span className="history-tag" data-testid="history-not-current">
              {editorText("Not in the project now")}</span>
          )}
        </span>
        <span className="history-when">{isNow ? <span className="history-now">{editorText("Now")}</span> : when(entry.at)}</span>
      </button>
      {open && (
        <div className="history-detail">
          {entry.lines.map((line, i) => (
            <div key={i} className="history-line">
              {line}
            </div>
          ))}
          {!entry.current && <div className="history-line muted">{editorText("Not in the project now: it was undone, or History went back past it.")}</div>}
          <div className="history-actions">
            <button className="btn" data-testid="history-restore" disabled={!online || isNow || busy || loading !== undefined} onClick={() => onRestore(entry.revision)}>
              <RotateCcw />
              {loading === entry.revision ? editorText('Loading…') : busy ? editorText('Applying change…') : isNow ? editorText("This is now") : editorText("Restore this version")}
            </button>
            <button className="btn ghost" onClick={() => onPick(entry.revision)}>
              {editorText("Close")}</button>
          </div>
        </div>
      )}
    </div>
  );
}

function TurnGroup({ group, turn, ...rest }: RowProps & { group: Group; turn: AgentTurn | undefined }) {
  useEditorLanguage();
  const newest = group.entries[0]!;
  const undo = turn?.undo;
  return (
    <div
      className={`history-turn ${group.entries.some((e) => e.current) ? "" : "out"}`}
      role="group"
      aria-label={editorText("Agent turn{0}", { 0: turn ? `: ${turn.prompt}` : "" })}
      data-testid="history-turn"
      data-turn={group.turnId}
      data-undo={undo?.state ?? "none"}
    >
      <button className="history-turn-head" disabled={!rest.readable || rest.busy} aria-expanded={rest.selected === newest.revision} title={editorText("Preview the project after this turn")} onClick={() => rest.onPick(newest.revision)}>
        <Sparkles />
        <span className="history-label">
          <span className="history-who">{editorText("Agent")}</span>
          <span className="history-what">{turn ? `“${turn.prompt}”` : editorText("A turn")}</span>
        </span>
        <span className="history-when">{when(newest.at)}</span>
      </button>
      {group.entries.map((e) => (
        <Entry key={e.revision} entry={e} nested {...rest} />
      ))}
      {undo && (undo.state === "available" || undo.state === "blocked" || undo.state === "undone") && (
        <div className="history-turn-foot">
          {undo.state === "undone" ? (
            <span className="muted small">{editorText("Turn undone")}</span>
          ) : (
            <button className="link-btn" data-testid="history-undo-turn" disabled={!rest.online || rest.busy || undo.state !== "available"} title={undo.reason ?? editorText("Undo this turn's edits, keeping later ones")} onClick={() => void actions.agentUndoTurn(turn!.id)}>
              <Undo2 />
              {editorText("Undo turn")}</button>
          )}
          {undo.state === "blocked" && undo.reason && <span className="muted small history-reason">{undo.reason}</span>}
        </div>
      )}
    </div>
  );
}

