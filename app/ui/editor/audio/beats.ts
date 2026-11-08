// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Beats on the timeline (D70): the cached beats of the media used by the timeline's items with
// sound, read from the server's analysis cache (analysis.cached, never a job: analysis is on
// demand, D66), and read again when a beats job for one of them finishes. The timeline draws them
// as ticks and, with snapping on, moves, trims and the playhead stick to them (edit.ts).
import { useContext, useEffect, useRef, useSyncExternalStore } from "react";

import type { BeatMap } from "@/app/ui/editor/timeline/edit";
import type { ProjectModel } from "@/app/lib/editor/state/project";
import {analysisVersions} from '@/app/lib/editor/analysis/versions'
import { hasSound } from "@/app/ui/editor/audio/sound";
import { actions, useApp } from "@/app/lib/editor/state/store";
import {currentEditorStore,EditorStoreContext} from '@/app/lib/editor/state/store/core';
import type {EditorStore} from '@/app/lib/editor/project-store';

/** A media file's beats analysis as the editor keeps it. */
export interface MediaBeats {
  /** null: no steady beat. */
  readonly bpm: number | null;
  readonly confidence: number;
  /** Seconds of the file. */
  readonly beats: ReadonlyArray<number>;
}

export interface BeatsState {
  readonly dir: string | undefined;
  /** By media id: the analysis, null when none is cached (or the media has no sound), "failed" when it couldn't be read (read again on reconnect; Find beats still works). Missing: being read. */
  readonly entries: ReadonlyMap<string, MediaBeats | null | "failed">;
  /** Beat times of the media that have some, for drawing and snapping. */
  readonly map: BeatMap;
}

let state: BeatsState = { dir: undefined, entries: new Map(), map: new Map() };
const listeners = new Set<() => void>();
const subscribe = (l: () => void) => (listeners.add(l), () => void listeners.delete(l));
export const useBeats = <T>(select: (s: BeatsState) => T): T => useSyncExternalStore(subscribe, () => select(state));
export const getBeats = () => state;

function setEntry(dir: string, mediaId: string, entry: MediaBeats | null | "failed") {
  if (dir !== state.dir) return;
  const entries = new Map(state.entries);
  entries.set(mediaId, entry);
  const map = new Map<string, ReadonlyArray<number>>();
  for (const [id, e] of entries) if (e && e !== "failed" && e.beats.length) map.set(id, e.beats);
  state = { ...state, entries, map };
  for (const l of listeners) l();
}

let versions='',epoch=0,owner:EditorStore|undefined;
function forProject(dir: string,nextVersions:string,nextOwner:EditorStore|undefined) {
  if (dir === state.dir&&versions===nextVersions&&owner===nextOwner) return;
  versions=nextVersions;owner=nextOwner;epoch++;
  state = { dir, entries: new Map(), map: new Map() };
  for (const l of listeners) l();
}

// Reads in flight (by dir and media), so a reconnect or a finished job doesn't read twice at once.
const reading = new Map<string,Promise<void>>();

async function load(dir: string, mediaId: string, fresh=false):Promise<void> {
  const revision=epoch,origin=owner;
  const current=()=>revision===epoch&&!!origin&&currentEditorStore()===origin&&owner===origin&&state.dir===dir;
  if(!current())return;
  const key = `${dir}\0${mediaId}\0${revision}`;
  const pending=reading.get(key);
  if(pending){await pending;if(fresh&&current())await load(dir,mediaId);return}
  const request=(async()=>{
    // The onsets stay on the server: the timeline only draws and snaps to beats.
    const result = (await actions.analysisCached(mediaId, "beats", ["onsets"])) as {hasAudio?:boolean;bpm?:number|null;confidence?:number;beats?:ReadonlyArray<number>}|null|undefined;
    if(!current())return;
    if(result===undefined)return setEntry(dir,mediaId,'failed');
    setEntry(dir, mediaId, result?.hasAudio ? { bpm: result.bpm ?? null, confidence: result.confidence ?? 0, beats: result.beats ?? [] } : null);
  })().finally(()=>{reading.delete(key)});
  reading.set(key,request);
  await request;
}

/** The media with sound used on the timeline. */
const soundMediaIds = (project: ProjectModel) => [...new Set(project.tracks.flatMap((t) => t.items.filter((i) => i.media && hasSound(project, i)).map((i) => i.media!)))].sort();

/** Keeps the beats of the timeline's media loaded for the open project. */
export function useBeatsLoading(project: ProjectModel, dir: string) {
  const editor=useContext(EditorStoreContext);
  const { analysisJobs, connection } = useApp();
  const online = connection === "connected";
  const media = soundMediaIds(project);
  const sourceVersions=analysisVersions(project),key = `${media.join(",")}|${sourceVersions}`;
  const seen = useRef(new Set<string>());
  useEffect(() => {forProject(dir,sourceVersions,editor);seen.current.clear()}, [dir,sourceVersions,editor]);
  useEffect(() => {
    if (!online || state.dir !== dir || currentEditorStore()!==editor || owner!==editor) return;
    // Not read yet, or a read that failed (read again when the connection is back).
    for (const id of media) if (!state.entries.has(id) || state.entries.get(id) === "failed") void load(dir, id);
  }, [online, dir, key,editor]);
  useEffect(() => {
    if(!online||currentEditorStore()!==editor||owner!==editor||state.dir!==dir)return;
    for (const job of analysisJobs) {
      if (job.projectDir !== dir || job.analyzer !== "beats" || job.phase !== "done") continue;
      for(const id of new Set([job.mediaId,...job.mediaIds])){
        const receipt=JSON.stringify([job.id,id]);
        if(!media.includes(id)||seen.current.has(receipt)||project.media.find(media=>media.id===id)?.path!==job.version)continue;
        seen.current.add(receipt);void load(dir,id,true);
      }
    }
  }, [analysisJobs, dir,key,online,editor]);
}
