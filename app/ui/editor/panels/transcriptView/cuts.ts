// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { decodeEnvelope } from "@/app/lib/editor/core/text/transcript";
import { useSyncExternalStore, useRef, useEffect, useContext } from "react";
import type { Plan } from "@/app/ui/editor/timeline/edit";
import { playback } from "@/app/ui/editor/preview/playback";
import { captureEditorFocus } from '@/app/ui/editor/preview/focus';
import type { ProjectModel } from "@/app/lib/editor/state/project";
import {analysisVersions} from '@/app/lib/editor/analysis/versions'
import { actions, getState, projectOf, useApp } from "@/app/lib/editor/state/store";
import { currentEditorStore, EditorStoreContext } from '@/app/lib/editor/state/store/core';
import { editorText } from '@/app/lib/editor/i18n';
import { joinWords } from '@/app/lib/editor/core/text/word-spacing';
import { transcriptStore, getTranscripts, selectedWords, wordsOfEdit, doneTranscripts, scopeOf, planCutWords, planCaptions, planRemovePauses, planRemoveFillers, mediaOnMain, type MediaTranscript, type TranscriptEntry, useTranscripts } from "@/app/ui/editor/panels/transcript";
import { pauseMin } from "@/app/ui/editor/panels/transcriptView/threshold";
import type { CaptionOptions } from '../transcript';

type CutContext = {
  readonly owner: NonNullable<ReturnType<typeof currentEditorStore>>;
  readonly project: ProjectModel;
  readonly revision: number;
  readonly transcript: ReturnType<typeof getTranscripts>;
  readonly selection: boolean;
  readonly speech: ReadonlyMap<string,string>;
};

const speechFingerprints=new WeakMap<MediaTranscript,string>();
function speechFingerprint(entry:TranscriptEntry|undefined):string {
  if(entry?.state!=='done')return entry?.state??'missing';
  let value=speechFingerprints.get(entry.transcript);
  if(value===undefined){
    const {words,segments,seconds,language}=entry.transcript;
    value=JSON.stringify([words,segments,seconds,language]);
    speechFingerprints.set(entry.transcript,value);
  }
  return value;
}

function currentCut(context:CutContext):boolean {
  const current=getTranscripts(),snapshot=context.owner.getSnapshot().snapshot;
  const before=context.transcript;
  return currentEditorStore()===context.owner&&current.owner===context.owner&&!!snapshot&&
    projectOf(snapshot)===context.project&&snapshot.revision===context.revision&&
    current.dir===before.dir&&current.model===before.model&&current.versions===before.versions&&current.source===before.source&&
    [...context.speech].every(([media,value])=>speechFingerprint(current.entries.get(media))===value)&&
    (!context.selection||(current.anchor===before.anchor&&current.focus===before.focus));
}

function readyCut(context:CutContext):boolean {
  if(currentCut(context))return true;
  if(currentEditorStore()===context.owner)context.owner.set({notice:{text:editorText('The project or transcript selection changed. Run the action again.')}});
  return false;
}

/** Send the captured transcript operation as one edit (one undo). */
async function sendCut(context:CutContext,plan: Plan | { readonly problem: string }) {
  if(!readyCut(context))return false;
  const applied = await actions.itemEdit(()=>currentCut(context)?plan:{problem:editorText('The project or transcript selection changed. Run the action again.')},{dir:context.transcript.dir,baseRevision:context.revision});
  const current=getTranscripts();
  if (applied&&currentEditorStore()===context.owner&&current.anchor===context.transcript.anchor&&current.focus===context.transcript.focus&&current.source===context.transcript.source) transcriptStore.clear();
  return applied;
}

/** How long a cut waits for a loudness analysis it started (as the agent's transcript_edit, D74). */
const LOUDNESS_WAIT_MS = 20_000;

/**
 * Before cutting: the loudness analysis (silences and speech envelope, D74) of these media, run
 * when it isn't cached and waited for up to LOUDNESS_WAIT_MS, then their transcripts re-read, so
 * the panel cuts where the agent's transcript_edit would. Without it (failed, an export holds the
 * queue, too slow) the cut uses the transcript's own edges.
 */
async function withLoudness(context:CutContext,mediaIds: ReadonlyArray<string>) {
  const dir = context.transcript.dir;
  const failures=loudnessFailed.get(context.owner)??new Set<string>();
  loudnessFailed.set(context.owner,failures);
  const failedKey=(id:string)=>JSON.stringify([id,context.project.media.find(media=>media.id===id)?.path]);
  const missing = mediaIds.filter((id) => {
    const e = getTranscripts().entries.get(id);
    return e?.state === "done" && !e.transcript.measured && !failures.has(failedKey(id));
  });
  if (!currentCut(context)||!dir || !missing.length) return;
  await Promise.all(
    missing.map(async (id) => {
      const startedAt = Date.now();
      if(!currentCut(context))return;
      const result=await actions.analyzeMedia(id,['loudness']);
      if(!currentCut(context))return;
      if(!result.ok){failures.add(failedKey(id));return}
      const jobs=new Set(result.value.flatMap(job=>job.jobId?[job.jobId]:[]));
      // Couldn't start (or a job of it failed / was stopped): not again for this media this session.
      const failed = () =>
        context.owner.getSnapshot().analysisJobs.some((j) => jobs.has(j.id) && (j.phase === "failed" || j.phase === "cancelled"));
      const deadline = startedAt + LOUDNESS_WAIT_MS;
      while (Date.now() < deadline) {
        if(!currentCut(context))return;
        if (failed()) {
          failures.add(failedKey(id));
          return;
        }
        if (await actions.analysisCached(id, "loudness", ["envelope"])) break;
        await new Promise((r) => setTimeout(r, 250));
      }
      if(currentCut(context))await load(dir, id);
    }),
  );
}

/** Failures belong to an editor instance and source version, not a reused media ID. */
const loudnessFailed = new WeakMap<object,Set<string>>();
/** A cut is being prepared (the loudness analysis it waits for): the buttons wait, ⌫ does nothing. */
const busyCuts = new WeakSet<object>();
const busyListeners = new Set<() => void>();
const setCutBusy = (owner:object,busy: boolean) => {
  if(busy)busyCuts.add(owner);else busyCuts.delete(owner);
  for (const l of busyListeners) l();
};
export const useCutBusy = () => {
  const owner=useContext(EditorStoreContext);
  return useSyncExternalStore((l) => (busyListeners.add(l), () => void busyListeners.delete(l)), () => !!owner&&busyCuts.has(owner));
};
/** Runs one cut at a time, marked busy while it waits. */
async function oneCut(run: (context:CutContext) => Promise<boolean>,selection=false) {
  const owner=currentEditorStore(),snapshot=owner?.getSnapshot().snapshot,transcript=getTranscripts();
  if(!owner||!snapshot||busyCuts.has(owner)||transcript.owner!==owner||transcript.dir!==snapshot.dir||!owner.getSnapshot().biliupCanEdit||owner.getSnapshot().connection!=='connected')return false;
  const project=projectOf(snapshot),scope=scopeOf(project,transcript.source);
  const media=scope?[scope.media]:mediaOnMain(project);
  const speech=new Map(media.map(id=>[id,speechFingerprint(transcript.entries.get(id))]));
  const context:CutContext={owner,project,revision:snapshot.revision,transcript,selection,speech};
  setCutBusy(owner,true);
  try {
    return await run(context);
  } catch(error) {
    if(currentEditorStore()===owner)owner.set({notice:{text:editorText(error instanceof Error?error.message:String(error))}});
    return false;
  } finally {
    setCutBusy(owner,false);
  }
}

/** ⌫ in the Transcript panel: cut the selected words. */
export const cutSelectedWords = () => oneCut(cutSelected,true);
async function cutSelected(context:CutContext) {
  const before = context.transcript,project=context.project;
  const keys = selectedWords(wordsOfEdit(project, doneTranscripts(before.entries), scopeOf(project, before.source)), before.anchor, before.focus);
  if (!keys.length) return false;
  await withLoudness(context,[...new Set(keys.map((w) => w.media))]);
  if(!readyCut(context))return false;
  const {entries}=getTranscripts(),{anchor,focus,source}=before;
  const transcripts = doneTranscripts(entries);
  const scope = scopeOf(project, source);
  const selected = selectedWords(wordsOfEdit(project, transcripts, scope), anchor, focus);
  if (!selected.length) return false;
  return sendCut(context,planCutWords(project, transcripts, selected, scope));
}

/** D264 (#77): the selected words as a chip in the composer (the stretch they're heard over, quoted). */
export function addSelectedWordsToChat(): boolean {
  const owner=currentEditorStore(),snapshot=owner?.getSnapshot().snapshot,before=getTranscripts();
  if (!owner||!snapshot||before.owner!==owner||before.dir!==snapshot.dir) return false;
  const project = projectOf(snapshot);
  const { entries, anchor, focus, source } = before;
  const transcripts = doneTranscripts(entries);
  const selected = selectedWords(wordsOfEdit(project, transcripts, scopeOf(project, source)), anchor, focus);
  if (!selected.length) return false;
  const text = joinWords(selected.map((w) => transcripts.get(w.media)?.words[w.index]?.text ?? ""));
  actions.addWordsToChat(Math.min(...selected.map((w) => w.start)), Math.max(...selected.map((w) => w.end)), text);
  return true;
}

/**
 * "Make captions" (D82): measured like a cut (the loudness pass, so words sit where they're heard),
 * then core planMakeCaptions for each transcribed media on main, one edit; the new captions are
 * selected so the Inspector shows them.
 */
export const makeCaptions = (options: CaptionOptions, expectedFixesDropped=0) => {
  const chosen = { ...options };
  return oneCut(context => makeAll(context,chosen,expectedFixesDropped));
};
async function makeAll(context:CutContext,options: CaptionOptions,expectedFixesDropped:number) {
  const unchanged=captureEditorFocus();
  await withLoudness(context,readMedia(context));
  if(!readyCut(context))return false;
  const plan = planCaptions(context.project, doneTranscripts(getTranscripts().entries), options, scopeOf(context.project, context.transcript.source));
  if(!('problem' in plan)&&plan.fixesDropped!==expectedFixesDropped){
    context.owner.set({notice:{text:editorText('The caption update changed. Reopen the caption options.')}});
    return false;
  }
  const applied = await sendCut(context,plan);
  if (applied && unchanged() && !("problem" in plan))playback.select(plan.items[0]);
  return applied;
}

/** Remove pauses / fillers: planned when clicked, on the project as it is then (not the render's). */
export const removeAll = (kind: "pauses" | "fillers") => oneCut(context => removeEvery(context,kind));
async function removeEvery(context:CutContext,kind: "pauses" | "fillers") {
  const threshold=pauseMin;
  await withLoudness(context,readMedia(context));
  if(!readyCut(context))return false;
  const project = context.project;
  const transcripts = doneTranscripts(getTranscripts().entries);
  const scope = scopeOf(project, context.transcript.source);
  return sendCut(context,kind === "pauses" ? planRemovePauses(project, transcripts, threshold, scope) : planRemoveFillers(project, transcripts, scope));
}

/** The media the tab reads now (D215: its source's, else the main track's). */
function readMedia(context:CutContext): Array<string> {
  const scope = scopeOf(context.project, context.transcript.source);
  return scope ? [scope.media] : mediaOnMain(context.project);
}

const transcriptLoads=new WeakMap<object,Map<string,symbol>>();
export async function load(dir: string, mediaId: string) {
  const owner=currentEditorStore(),snapshot=owner?.getSnapshot().snapshot;
  const before=getTranscripts();
  if(!owner||!snapshot||snapshot.dir!==dir||before.dir!==dir||before.owner!==owner)return;
  const version=projectOf(snapshot).media.find(media=>media.id===mediaId)?.path;
  if(!version)return;
  const requests=transcriptLoads.get(owner)??new Map<string,symbol>();
  transcriptLoads.set(owner,requests);
  const request=Symbol();
  requests.set(mediaId,request);
  const current=()=>{
    const now=getTranscripts(),latest=owner.getSnapshot().snapshot;
    return currentEditorStore()===owner&&now.owner===owner&&now.dir===dir&&now.model===before.model&&now.versions===before.versions&&
      requests.get(mediaId)===request&&!!latest&&projectOf(latest).media.find(media=>media.id===mediaId)?.path===version;
  };
  if (getTranscripts().entries.get(mediaId)?.state !== "done") transcriptStore.setEntry(dir, mediaId, { state: "loading" });
  const result = (await actions.analysisCached(mediaId, "transcript")) as { hasAudio?: boolean; words?: MediaTranscript["words"]; segments?: MediaTranscript["segments"]; language?: string; seconds?: number } | null | undefined;
  if(!current())return;
  let entry: TranscriptEntry;
  if (result === undefined) entry = { state: "error", message: "Couldn't read the transcript." };
  else if (result === null) entry = { state: "none" };
  else if (!result.hasAudio) entry = { state: "silent" };
  else {
    const loud = (await actions.analysisCached(mediaId, "loudness")) as { hasAudio?: boolean; silence?: { ranges?: MediaTranscript["silences"] }; envelope?: unknown } | null | undefined;
    entry = {
      state: "done",
      transcript: {
        words: result.words ?? [],
        segments: result.segments,
        language: result.language,
        seconds: result.seconds,
        silences: loud?.hasAudio ? loud.silence?.ranges : undefined,
        envelope: loud?.hasAudio ? decodeEnvelope(loud.envelope) : undefined,
        measured: !!loud,
      },
    };
  }
  if(current()){
    requests.delete(mediaId);
    const displayed=getTranscripts(),previous=displayed.entries.get(mediaId),latest=owner.getSnapshot().snapshot;
    // A new transcription can reuse the same word indices for different speech. Loudness-only
    // refreshes preserve the selection, as do failures and updates to another source's words.
    if(latest&&displayed.anchor!==undefined&&previous?.state==='done'&&(entry.state==='done'||entry.state==='silent')
      &&speechFingerprint(previous)!==speechFingerprint(entry)){
      const project=projectOf(latest);
      const selected=selectedWords(wordsOfEdit(project,doneTranscripts(displayed.entries),scopeOf(project,displayed.source)),displayed.anchor,displayed.focus);
      if(selected.some(word=>word.media===mediaId))transcriptStore.clear();
    }
    transcriptStore.setEntry(dir, mediaId, entry);
  }
}

/** Loads the transcripts of the media the tab reads (the main track's, or its source's), and again when a transcript job for one finishes. */
export function useTranscriptLoading(project: ProjectModel, dir: string) {
  const owner=useContext(EditorStoreContext);
  const { analysisJobs, connection, transcription } = useApp();
  const model=transcription?.selected;
  const versions=analysisVersions(project);
  const entries = useTranscripts((s) => s.entries);
  const source = useTranscripts((s) => s.source);
  const online = connection === "connected";
  const scope = scopeOf(project, source);
  const media = scope ? [scope.media] : mediaOnMain(project);
  const seenJobs = useRef(new Set<string>());
  // Failed reads are tried again once each time the tab opens (and with Retry).
  const retried = useRef(new Set<string>());
  useEffect(() => {transcriptStore.forProject(dir,model,versions,owner);retried.current.clear();seenJobs.current.clear()}, [owner,dir,model,versions]);
  useEffect(() => {
    if (!online || !model || getTranscripts().dir !== dir||getTranscripts().owner!==owner) return;
    for (const id of media) {
      const e = entries.get(id);
      if (!e) void load(dir, id);
      else if (e.state === "error" && !retried.current.has(id)) {
        retried.current.add(id);
        void load(dir, id);
      }
    }
  }, [owner,online, dir, model,versions, media.join(","), entries]);
  useEffect(() => {
    if(!online||!model||currentEditorStore()!==owner||getTranscripts().owner!==owner||getTranscripts().dir!==dir)return;
    const reload=new Set<string>();
    for (const job of analysisJobs) {
      if (job.projectDir !== dir || job.phase !== "done" || (job.analyzer!=="transcript"&&job.analyzer!=="loudness")) continue;
      for (const id of new Set([job.mediaId, ...job.mediaIds])) {
        if (!media.includes(id)) continue;
        if(project.media.find(media=>media.id===id)?.path!==job.version)continue;
        const receipt=JSON.stringify([job.id,id]);
        if(seenJobs.current.has(receipt))continue;
        const state = getTranscripts().entries.get(id)?.state;
        // A source outside the current view has not consumed this job. An in-flight read may
        // predate completion, so wait for that read and then fetch the newly finished result.
        if(job.analyzer==="transcript"?state==="loading":state!=="done")continue;
        seenJobs.current.add(receipt);reload.add(id);
      }
    }
    for(const id of reload)void load(dir,id);
  }, [owner,online,model,analysisJobs,dir,versions,media.join(","),entries]);
}
