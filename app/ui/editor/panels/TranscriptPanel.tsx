import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { wordSeparator } from '@/app/lib/editor/core/text/word-spacing';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Transcript panel (D32, D69; a left rail tab since U11, D177): what's said on the main track, clip by clip in edit order (what
// you'd hear), or (D215) on an item off it (a source picker lists them, a take read from its
// microphone; selecting one on the timeline reads it), as selectable words. The word under the playhead is highlighted while it plays;
// clicking a word goes there (and selects it); dragging or ⇧-clicking selects more, and ⌫/Delete
// cuts the selection through core planTextCut as one undoable edit (a bar shows the selection, with
// Cut and clear). The tools (#59, D231) are rows in one card: Fillers (hesitations only; "maybe" ones
// are only marked) and "Pauses over [x s]" do the same for the whole edit, each row saying what it
// would cut; Captions (D82) makes captions from the transcripts that follow the cuts (mode and look
// picked in the shared, window-clamped Popover). Media without a transcript can be transcribed
// here (the Library's analysis job, with its progress); a missing whisper.cpp or speech model
// points to Setup.
import { fillersOf, pausesOf, type EditWord, type Filler, type Pause } from "@/app/lib/editor/core/text/transcript";
import { AtSign, Captions, Scissors, X } from "lucide-react";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import { memo, useCallback, useEffect, useMemo, useRef } from "react";

import { frameTimecode, timecode } from "@/app/lib/editor/state/format";
import { playback, usePlayback } from "@/app/ui/editor/preview/playback";
import { layoutOf, type ProjectModel } from "@/app/lib/editor/state/project";
import { actions, getState, useApp } from "@/app/lib/editor/state/store";
import {currentEditorStore,projectOf} from '@/app/lib/editor/state/store/core';
import {beginEditorViewGesture} from '../workspace/pointerGesture';
import { doneTranscripts, getTranscripts, mainSpeechMedia, mediaOnMain, scopeKey, scopeOf, sourceChoices, planRemoveFillers, planRemovePauses, selectedWords, transcriptStore, useTranscripts, wordKey, wordsOfEdit, type MediaTranscript } from "@/app/ui/editor/panels/transcript";
import { useTranscriptLoading, load, useCutBusy, cutSelectedWords, removeAll, addSelectedWordsToChat } from "@/app/ui/editor/panels/transcriptView/cuts";
import { useThreshold, PAUSE_SHOWN_S, PAUSE_STEPS } from "@/app/ui/editor/panels/transcriptView/threshold";
import { Untranscribed, CaptionsButton, PlayheadWord } from "@/app/ui/editor/panels/transcriptView/parts";
export { cutSelectedWords, makeCaptions, removeAll } from "@/app/ui/editor/panels/transcriptView/cuts";

const EMPTY: ReadonlyArray<EditWord> = [];
type Marks = { readonly fillers: ReadonlyMap<number, Filler["kind"]>; readonly pauses: ReadonlyMap<number, Pause> };

/** One clip's words; re-renders only when its words, marks, threshold or selected span change. */
const ClipWords = memo(function ClipWords({ own, transcript, marks, threshold, selLo, selHi, fps, onWordDown }: { own: ReadonlyArray<EditWord>; transcript: MediaTranscript; marks: Marks | undefined; threshold: number; selLo: number; selHi: number; fps: number; onWordDown:(event:React.PointerEvent,word:EditWord)=>void }) {
  useEditorLanguage();
  return (
    <p className="tx-words">
      {own.map((w, i) => {
        const filler = marks?.fillers.get(w.index);
        const pause = marks?.pauses.get(w.index);
        const next = own[i + 1];
        const key = wordKey(w);
        const sel = i >= selLo && i <= selHi && selLo !== -1;
        return (
          <span key={key}>
            <span
              className={`tx-word ${filler ? `filler ${filler}` : ""} ${sel ? "sel" : ""}`}
              data-testid="tx-word"
              data-key={key}
              data-clip={w.clip}
              data-index={w.index}
              data-cycle={w.cycle}
              data-start={w.start}
              data-end={w.end}
              data-filler={filler}
              title={filler === "hesitation" ? editorText("A hesitation: Remove fillers cuts it") : filler === "maybe" ? editorText("Maybe a filler: cut it by hand if it adds nothing") : frameTimecode(w.start, fps)}
              onPointerDown={(e) => onWordDown(e, w)}
            >
              {(transcript.words[w.index]?.text ?? "").trim()}
            </span>
            {pause && next && next.cycle === w.cycle && next.index === w.index + 1 ? (
              <span className={`tx-pause ${pause.end - pause.start >= threshold ? "long" : ""}`} data-testid="tx-pause" data-seconds={(pause.end - pause.start).toFixed(2)} title={editorText("A {0} s pause", { 0: (pause.end - pause.start).toFixed(1) })}>
                {(pause.end - pause.start).toFixed(1)}{editorText("s")}</span>
            ) : (
              next?wordSeparator(transcript.words[w.index]?.text??'',transcript.words[next.index]?.text??''):''
            )}
          </span>
        );
      })}
    </p>
  );
});

export function TranscriptPanel({ project, dir }: { project: ProjectModel; dir: string }) {
  useEditorLanguage();
  const { connection, setupChecks, setupChecking, setupError, transcription, transcriptionError, analysisError, biliupCanEdit } = useApp();
  const online = connection === "connected";
  useTranscriptLoading(project, dir);
  useEffect(() => {
    if (online&&!getState().setupChecks) void actions.checkSetup();
  }, [online]);
  const entries = useTranscripts((s) => s.entries);
  const anchor = useTranscripts((s) => s.anchor);
  const focus = useTranscripts((s) => s.focus);
  // The timeline shows next to the transcript (U11): one selection at a time, so ⌫ means one thing.
  const timelineSelected = usePlayback((s) => s.selected);
  useEffect(() => {
    if (anchor !== undefined) playback.select(undefined);
  }, [anchor]);
  useEffect(() => {
    if (timelineSelected !== undefined) transcriptStore.clear();
  }, [timelineSelected]);
  const transcripts = useMemo(() => doneTranscripts(entries), [entries]);
  // D215: the main track, or an item off it (the picker, or the item selected on the timeline).
  const source = useTranscripts((s) => s.source);
  const scope = useMemo(() => scopeOf(project, source), [project, source]);
  const choices = useMemo(() => sourceChoices(project), [project]);
  useEffect(() => {
    if (timelineSelected === undefined) return;
    const item = project.tracks.flatMap((t) => t.items).find((i) => i.id === timelineSelected);
    if (!item || item.type === "text" || item.type === "captions") return;
    const own = scopeOf(project, item.id);
    const onMain = project.tracks.some((t) => t.role === "main" && t.items.some((i) => i.id === item.id || i.id === item.linkedTo));
    if (own) transcriptStore.setSource(item.id);
    else if (onMain) transcriptStore.setSource(undefined);
  }, [timelineSelected]);
  const words = useMemo(() => wordsOfEdit(project, transcripts, scope), [project, transcripts, scope]);
  const selected = selectedWords(words, anchor, focus);
  const byClip = useMemo(() => {
    const out = new Map<string, Array<EditWord>>();
    for (const w of words) (out.get(w.clip) ?? out.set(w.clip, []).get(w.clip)!).push(w);
    return out;
  }, [words]);
  // The selection per clip as a span of positions in its word list, so rows compare primitives.
  const selIn = (clip: string) => {
    const list = byClip.get(clip);
    if (!list || !selected.length) return [-1, -1] as const;
    const keys = new Set(selected.map(wordKey));
    let lo = -1;
    let hi = -1;
    list.forEach((w, i) => {
      if (keys.has(wordKey(w))) {
        if (lo === -1) lo = i;
        hi = i;
      }
    });
    return [lo, hi] as const;
  };
  const [threshold, setThreshold] = useThreshold();
  const pausePlan = useMemo(() => planRemovePauses(project, transcripts, threshold, scope), [project, transcripts, threshold, scope]);
  const fillerPlan = useMemo(() => planRemoveFillers(project, transcripts, scope), [project, transcripts, scope]);
  const fps = project.canvas.fps;
  const media = scope ? [scope.media] : mediaOnMain(project);
  const whisperReady = setupChecks?.find((c) => c.id === "whisper")?.status === "ready";
  const model = transcription?.models.find((m) => m.id === transcription.selected);

  // Fillers and pauses per media (index → kind / the pause after it), derived like transcript_read's.
  const marks = useMemo(() => {
    const out = new Map<string, { fillers: Map<number, Filler["kind"]>; pauses: Map<number, Pause> }>();
    for (const [id, t] of transcripts) {
      const fillers = new Map<number, Filler["kind"]>();
      for (const f of fillersOf(t.words, t.language)) for (let k = 0; k < f.n; k++) fillers.set(f.i + k, f.kind);
      const pauses = new Map<number, Pause>();
      for (const p of pausesOf(t.words, t.seconds, t.silences, PAUSE_SHOWN_S, t.segments, t.envelope)) if (!p.edge) pauses.set(p.after, p);
      out.set(id, { fillers, pauses });
    }
    return out;
  }, [transcripts]);

  const bodyRef = useRef<HTMLDivElement>(null);
  const stopSelecting=useRef<(()=>void)|undefined>(undefined);
  useEffect(()=>{stopSelecting.current?.();return()=>stopSelecting.current?.()},[project,dir,source,entries]);
  const onWordDown=useCallback((event:React.PointerEvent,word:EditWord)=>{
    if(event.button!==0)return;
    event.preventDefault();stopSelecting.current?.();
    const owner=currentEditorStore(),snapshot=owner?.getSnapshot().snapshot,before=getTranscripts();
    if(!owner||!snapshot||snapshot.dir!==dir||projectOf(snapshot)!==project||before.owner!==owner||before.dir!==dir)return;
    const key=wordKey(word),keys=new Set(words.map(wordKey));
    if(!keys.has(key))return;
    if(event.shiftKey&&before.anchor!==undefined&&keys.has(before.anchor)){transcriptStore.extend(key);return}
    const current=()=>{
      const now=getTranscripts();
      return now.owner===owner&&now.dir===dir&&now.source===before.source&&now.model===before.model&&now.versions===before.versions&&now.entries===before.entries&&now.anchor===key;
    };
    const extend=(next:PointerEvent)=>{
      if(!current()){stopSelecting.current?.();return}
      const element=document.elementFromPoint(next.clientX,next.clientY)?.closest<HTMLElement>('[data-testid="tx-word"]');
      const target=element?.dataset.key;
      if(element&&bodyRef.current?.contains(element)&&target&&keys.has(target))transcriptStore.extend(target);
    };
    const finish=()=>{if(stopSelecting.current===cancel)stopSelecting.current=undefined};
    const cancel=beginEditorViewGesture(event,{move:extend,commit:next=>{extend(next);finish()},cancel:finish});
    if(!cancel)return;
    stopSelecting.current=cancel;
    transcriptStore.select(key);playback.seek(word.start);
  },[project,dir,words]);

  const main = project.tracks.find((t) => t.role === "main");
  const shownUntranscribed = new Set<string>();
  // The blocks: each main clip in edit order, or (D215) each item of the source in timeline order.
  const { starts } = layoutOf(project);
  const rows = scope
    ? scope.items.flatMap((id) => {
        const at = project.tracks.flatMap((tr) => tr.items.map((i) => ({ i, tr }))).find((x) => x.i.id === id);
        return at ? [{ item: at.i, start: starts.get(id) ?? 0, mediaId: scope.media as string | undefined, where: at.tr.name as string | undefined }] : [];
      })
    : (main?.items ?? []).map((item) => ({ item, start: starts.get(item.id) ?? 0, mediaId: item.type === "clip" && item.media ? mainSpeechMedia(project, item) : undefined, where: undefined as string | undefined }));
  const blocks = rows.map(({ item, start, mediaId, where }) => {
    const m = mediaId ? project.media.find((x) => x.id === mediaId) : undefined;
    if (!m || !media.includes(m.id)) return null;
    const entry = entries.get(m.id);
    const head = (
      <div className="tx-head">
        <span className="ellipsis">
          {item.name ?? m.name}
          {where ? <span className="muted">, {where}</span> : null}
        </span>
        <span className="mono">{timecode(start, fps)}</span>
      </div>
    );
    if (entry?.state === "done") {
      const own = byClip.get(item.id) ?? EMPTY;
      const [lo, hi] = selIn(item.id);
      return (
        <div key={item.id} className="tx-block" data-testid="tx-block" data-clip={item.id} data-media={m.id}>
          {head}
          {own.length ? <ClipWords own={own} transcript={entry.transcript} marks={marks.get(m.id)} threshold={threshold} selLo={lo} selHi={hi} fps={fps} onWordDown={onWordDown} /> : <div className="tx-note muted">{editorText("No speech in this part.")}</div>}
        </div>
      );
    }
    const first = !shownUntranscribed.has(m.id);
    shownUntranscribed.add(m.id);
    return (
      <div key={item.id} className="tx-block" data-testid="tx-block" data-clip={item.id} data-media={m.id}>
        {head}
        {entry?.state === "silent" ? (
          <div className="tx-note muted">{editorText("No sound to transcribe.")}</div>
        ) : !entry || entry.state === "loading" ? (
          <div className="tx-note muted">{editorText("Loading…")}</div>
        ) : entry.state === "error" ? (
          <div className="tx-note muted">
            {entry.message}{" "}
            <button className="link-btn" data-testid="tx-retry" onClick={() => void load(dir, m.id)}>
              {editorText("Retry")}</button>
          </div>
        ) : first ? (
          <Untranscribed mediaId={m.id} name={m.name} online={online} whisperReady={whisperReady} checking={!setupChecks||!model} modelMissing={model ? !model.installed : false} modelLabel={model ? `${model.name}, ${Math.round(model.bytes / 1e6)} MB` : ""} />
        ) : (
          <div className="tx-note muted">{editorText("Not transcribed yet (see above).")}</div>
        )}
      </div>
    );
  });

  const pauseCount = "problem" in pausePlan ? 0 : pausePlan.pauses;
  const fillerCount = "problem" in fillerPlan ? 0 : fillerPlan.fillers;
  const secs = (p: typeof pausePlan) => ("problem" in p ? "" : `${(p.framesRemoved / fps).toFixed(1)}\u00a0s`);
  const anyDone = transcripts.size > 0;
  const locked = !scope && main?.locked === true;
  const busy = useCutBusy();
  const status = locked ? editorText('The main track is locked: unlock it to cut by text.') : busy ? editorText('Measuring the sound before cutting…') : selected.length ? editorText('{0} words selected',{0:selected.length}) : '';
  return (
    <aside className="library tx-panel" data-testid="transcript-panel">
      <div className="lib-head">{editorText("Transcript")}</div>
      {setupError&&<div className="inline-error" role="alert">{setupError} <button className="link-btn" disabled={!online||setupChecking} onClick={()=>void actions.checkSetup()}>{editorText('Retry')}</button></div>}
      {!transcription&&transcriptionError&&<div className="inline-error" role="alert">{transcriptionError} <button className="link-btn" disabled={!online||setupChecking} onClick={()=>void actions.checkSetup()}>{editorText('Retry')}</button></div>}
      {(choices.length > 0 || scope) && (
        <div className="tx-source">
          <select
            className="tx-select"
            data-testid="tx-source"
            title={editorText("What to read")}
            value={scope ? (choices.find((c) => scopeKey(c.scope) === scopeKey(scope))?.id ?? "main") : "main"}
            onChange={(e) => transcriptStore.setSource(e.target.value === "main" ? undefined : e.target.value)}
          >
            <option value="main">{editorText("Main track")}</option>
            {choices.map((c) => (
              <option key={c.id} value={c.id}>
                {editorText(c.label)}
              </option>
            ))}
          </select>
        </div>
      )}
      {media.length > 0 && (
        <div className="tx-tools" data-testid="tx-tools">
          <div className="tx-tool">
            <div className="tx-tool-text">
              <div className="tx-tool-title">{editorText("Fillers")}</div>
              <div className="tx-tool-meta">{!anyDone ? editorText("um, uh, yyy") : fillerCount ? editorText("{0} found, {1}", { 0: fillerCount, 1: secs(fillerPlan) }) : editorText("None in the edit")}</div>
            </div>
            <button
              className="btn"
              data-testid="remove-fillers"
              data-count={fillerCount}
              aria-label={editorText("Remove fillers")}
              disabled={!online || !biliupCanEdit || busy || locked || !fillerCount}
              title={locked ? editorText("The main track is locked") : fillerCount ? editorText('Cut {0} hesitations, {1}. Words marked "maybe" stay.', { 0: fillerCount, 1: secs(fillerPlan) }) : editorText('No hesitation sounds to remove')}
              onClick={() => void removeAll("fillers")}
            >
              {editorText("Remove")}</button>
          </div>
          <div className="tx-tool">
            <div className="tx-tool-text">
              <label className="tx-tool-title tx-pause-label">
                {editorText("Pauses over")}<select className="tx-select tx-threshold" data-testid="pause-threshold" value={threshold} aria-label={editorText("Shortest pause to remove")} onChange={(e) => setThreshold(Number(e.target.value))}>
                  {PAUSE_STEPS.map((s) => (
                    <option key={s} value={s}>
                      {s} {editorText(" s")}</option>
                  ))}
                </select>
              </label>
              <div className="tx-tool-meta">{!anyDone ? editorText("Gaps between words") : pauseCount ? editorText("{0} found, {1}", { 0: pauseCount, 1: secs(pausePlan) }) : editorText("None in the edit")}</div>
            </div>
            <button
              className="btn"
              data-testid="remove-pauses"
              data-count={pauseCount}
              aria-label={editorText("Remove pauses")}
              disabled={!online || !biliupCanEdit || busy || locked || !pauseCount}
              title={locked ? editorText("The main track is locked") : pauseCount ? editorText("Cut {0} pause{1} of at least {2} s ({3}), keeping a little quiet next to the words", { 0: pauseCount, 1: pauseCount === 1 ? "" : "s", 2: threshold, 3: secs(pausePlan) }) : editorText("No pauses of {0} s or more in the edit", { 0: threshold })}
              onClick={() => void removeAll("pauses")}
            >
              {editorText("Remove")}</button>
          </div>
          <CaptionsButton project={project} media={media} disabled={!online || !biliupCanEdit || !anyDone} busy={busy} transcribed={anyDone} />
        </div>
      )}
      <div className={`tx-selbar ${status ? "on" : ""} ${selected.length && !locked && !busy ? "sel" : ""}`}>
        <span className="tx-status" data-testid="tx-status">
          {status}
        </span>
        {selected.length > 0 && (
          <>
            <button className="btn ghost" data-testid="tx-cut" disabled={!online || !biliupCanEdit || busy || locked} title={editorText("Cut the selected words (⌫)")} onClick={() => void cutSelectedWords()}>
              <Scissors />
              {editorText("Cut")}</button>
            {/* D264 (#77): the words as a chip in the Agent tab's composer. */}
            <Tooltip label={editorText("Add these words to chat")} shortcut="addToChat">
              <button className="icon-btn" data-testid="tx-add-to-chat" onClick={() => addSelectedWordsToChat()}>
                <AtSign />
              </button>
            </Tooltip>
            <Tooltip label={editorText("Clear the selection")} keys={["Esc"]}>
              <button className="icon-btn" data-testid="tx-clear" aria-label={editorText("Clear the selection")} onClick={() => transcriptStore.clear()}>
                <X />
              </button>
            </Tooltip>
          </>
        )}
      </div>
      <div
        className="tx-body"
        ref={bodyRef}
        onPointerDown={(e) => {
          if (e.target === e.currentTarget){stopSelecting.current?.();transcriptStore.clear()}
        }}
      >
        {analysisError && (
          <div className="inline-error small tx-error" data-testid="tx-error">
            {analysisError}
            <button className="link-btn" onClick={actions.dismissAnalysisError}>
              {editorText("Dismiss")}</button>
          </div>
        )}
        {media.length === 0 ? (
          <div className="empty" data-testid="tx-empty">
            <div className="empty-icon">
              <Captions />
            </div>
            <div className="empty-title">{editorText("Nothing to read yet")}</div>
            <div className="empty-sub">{scope ? editorText("No speech here.") : editorText("Put a clip with speech on the main track: its words show here, and cutting a word cuts the video.")}</div>
          </div>
        ) : (
          <div className="tx-doc">{blocks}</div>
        )}
        <PlayheadWord words={words} container={bodyRef} />
      </div>
    </aside>
  );
}
