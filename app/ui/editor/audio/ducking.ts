// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Ducking in the editor (D85): the loudness envelopes (D74) of the media on the key tracks of every
// ducked track (and on tracks with noise removal: their noise floor, D246), read from the server's
// analysis cache (analysis.cached), and the duck curves core
// computes from them (ducking.ts duckCurves) for the preview's mix (audio.ts adds the curve to each
// item's gain, as the export does) and the timeline (the duck line on the ducked track's items).
// A key media that hasn't been analysed gets its loudness analysis started once (the Library's
// Analyze job, on demand like D66, here because ducking asked for it), and is read again when that
// job finishes; meanwhile the Mixer says the ducking waits for it.
import { duckCurves, duckMediaIds, type DuckCurve } from "@/app/lib/editor/core/audio/ducking";
import { denoiseMediaIds } from "@/app/lib/editor/core/audio/effects";
import { decodeEnvelope, type SpeechEnvelope } from "@/app/lib/editor/core/text/transcript";
import { useContext, useEffect, useRef, useSyncExternalStore } from "react";

import type { ProjectModel } from "@/app/lib/editor/state/project";
import {analysisVersions} from '@/app/lib/editor/analysis/versions'
import { useLoudnessMeasuring } from "@/app/ui/editor/audio/loudnessPreview";
import { actions, useApp } from "@/app/lib/editor/state/store";
import {currentEditorStore,EditorStoreContext} from '@/app/lib/editor/state/store/core';
import type {EditorStore} from '@/app/lib/editor/project-store';

export interface DuckState {
  readonly dir: string | undefined;
  /** By media id: the envelope, null when analysed without sound. Missing: not known (yet). */
  readonly envelopes: ReadonlyMap<string, SpeechEnvelope | null>;
  /** Media whose loudness analysis this session started for ducking (each once). */
  readonly measuring: ReadonlySet<string>;
}

let state: DuckState = { dir: undefined, envelopes: new Map(), measuring: new Set() };
const listeners = new Set<() => void>();
const subscribe = (l: () => void) => (listeners.add(l), () => void listeners.delete(l));
const emit = () => {
  for (const l of listeners) l();
};
export const useDuckState = <T>(select: (s: DuckState) => T): T => useSyncExternalStore(subscribe, () => select(state));
export const getDuckState = () => state;

// The curves of the last project asked for, by its object and the envelopes: the preview asks every
// frame with the same object. A draft (a drag) is a new object and recomputes, which is cheap (about a
// millisecond per minute of key audio: a 10 ms grid).
let memo: { project: ProjectModel; envelopes: ReadonlyMap<string, SpeechEnvelope | null>; curves: Map<string, DuckCurve> } | undefined;
const NONE = new Map<string, DuckCurve>();
/** Every ducked track's curve for a project (memoised on the project object and the envelopes; empty at once when nothing ducks). */
export function duckCurvesOf(project: ProjectModel): Map<string, DuckCurve> {
  if (memo && memo.project === project && memo.envelopes === state.envelopes) return memo.curves;
  if (!project.tracks.some((t) => t.mixer?.effects?.some((fx) => fx.module === "audio.duck"))) return NONE;
  const curves = duckCurves(project, state.envelopes);
  memo = { project, envelopes: state.envelopes, curves };
  return curves;
}
/** The same for components: recomputed when the project or the envelopes change. */
export function useDuckCurves(project: ProjectModel): Map<string, DuckCurve> {
  useDuckState((s) => s.envelopes);
  return duckCurvesOf(project);
}

let versions='',epoch=0,owner:EditorStore|undefined;
function forProject(dir: string,nextVersions:string,nextOwner:EditorStore|undefined) {
  if (dir === state.dir&&versions===nextVersions&&owner===nextOwner) return;
  versions=nextVersions;owner=nextOwner;epoch++;
  state = { dir, envelopes: new Map(), measuring: new Set() };
  memo = undefined;
  emit();
}

const reading = new Map<string,Promise<void>>();
/** Read a media's loudness from the cache; start its analysis once when there's none. */
async function load(dir: string, mediaId: string, measure: boolean,fresh=false):Promise<void> {
  const revision=epoch,origin=owner;
  const current=()=>revision===epoch&&!!origin&&currentEditorStore()===origin&&owner===origin&&state.dir===dir;
  if(!current())return;
  const key = `${dir}\0${mediaId}\0${revision}`;
  const pending=reading.get(key);
  if(pending){await pending;if(fresh&&current())await load(dir,mediaId,measure);return}
  const request=(async()=>{
    const result = (await actions.analysisCached(mediaId, "loudness")) as {hasAudio?:boolean;envelope?:unknown}|null|undefined;
    if(!current()||result===undefined)return;
    if(result===null){
      if(measure&&origin?.getSnapshot().biliupCanEdit&&!state.measuring.has(mediaId)){
        state={...state,measuring:new Set([...state.measuring,mediaId])};emit();
        const started=await actions.analyzeMedia(mediaId);
        if(!started.ok&&current()){
          state={...state,measuring:new Set([...state.measuring].filter(id=>id!==mediaId))};emit();
        }
      }
      return;
    }
    const envelopes=new Map(state.envelopes);
    envelopes.set(mediaId,result.hasAudio?(decodeEnvelope(result.envelope)??null):null);
    state={...state,envelopes};emit();
  })().finally(()=>{reading.delete(key)});
  reading.set(key,request);
  await request;
}

/** Keeps the envelopes ducking needs loaded for the open project (mounted by the Editor), and the preview's At target gain measured (D246). */
export function useDuckLoading(project: ProjectModel, dir: string) {
  const editor=useContext(EditorStoreContext);
  const { analysisJobs, exportJobs, connection, biliupCanEdit } = useApp();
  const envelopes = useDuckState((s) => s.envelopes);
  useLoudnessMeasuring(project, envelopes);
  const online = connection === "connected";
  // Ducking's key media, and noise removal's (its track's noise floor, D88/D246).
  const media = [...new Set([...duckMediaIds(project), ...denoiseMediaIds(project)])].sort();
  const sourceVersions=analysisVersions(project),key = `${media.join(",")}|${sourceVersions}`;
  const seen = useRef(new Set<string>());
  useEffect(() => {forProject(dir,sourceVersions,editor);seen.current.clear()}, [dir,sourceVersions,editor]);
  useEffect(() => {
    if (!online || state.dir !== dir || currentEditorStore()!==editor || owner!==editor) return;
    for (const id of media) if (!state.envelopes.has(id)) void load(dir, id, true);
  }, [online, dir, key,editor,biliupCanEdit]);
  useEffect(() => {
    if(!online||currentEditorStore()!==editor||owner!==editor||state.dir!==dir)return;
    for (const job of analysisJobs) {
      if (job.projectDir !== dir || job.analyzer !== "loudness" || job.phase !== "done") continue;
      for(const id of new Set([job.mediaId,...job.mediaIds])){
        const receipt=JSON.stringify([job.id,id]);
        if((!media.includes(id)&&!state.envelopes.has(id))||seen.current.has(receipt)||project.media.find(media=>media.id===id)?.path!==job.version)continue;
        seen.current.add(receipt);void load(dir,id,false,true);
      }
    }
  }, [analysisJobs, dir, key,online,editor]);
  // An export measures missing envelopes itself (D85: the analysis queue waits for exports) and caches
  // them without a loudness job: when one of this project's exports ends, read the missing ones again.
  const exportsEnded = exportJobs.filter((j) => j.projectDir === dir && (j.phase === "done" || j.phase === "failed" || j.phase === "cancelled")).map((j) => j.id).join(",");
  useEffect(() => {
    if (!online || state.dir !== dir || !exportsEnded || currentEditorStore()!==editor || owner!==editor) return;
    for (const id of media) if (!state.envelopes.has(id)) void load(dir, id, false,true);
  }, [exportsEnded, online, dir, key,editor]);
}
