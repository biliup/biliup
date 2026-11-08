// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { EditorCommands } from '@/app/lib/editor/commands';
import { set, state, projectOf, currentEditorStore } from "@/app/lib/editor/state/store/core";
import { call, plainError } from "@/app/lib/editor/state/store/server";
import { pendingSaves } from '../edits';
import { editorText } from '@/app/lib/editor/i18n';
import type {Result} from '@/app/lib/editor/state/connection';
import type {EditorStore} from '@/app/lib/editor/project-store';
import {activeExport} from '@/app/lib/editor/export/jobs';

type ExportOrigin = { readonly dir: string; readonly baseRevision: number };
type ExportRefusal = { readonly message: string };

/** An export uses the displayed saved revision and cannot overtake queued edits. */
async function submitExport<T>(origin: ExportOrigin, run: () => Promise<T>): Promise<T | ExportRefusal> {
  const owner = currentEditorStore(), snapshot = state.snapshot;
  if (!owner || !snapshot || snapshot.dir !== origin.dir || snapshot.revision !== origin.baseRevision) return {message: editorText('The project changed. Try the action again.')};
  if (state.connection !== 'connected' || !state.biliupCanEdit) return {message: editorText('Editing permission is required to export.')};
  if (state.exportSubmitting) return {message: editorText('An export request is already pending.')};
  if (pendingSaves() || state.biliupSaving || state.biliupDirty || state.biliupSaveConflict) return {message: editorText('Save the current edits before exporting.')};
  owner.set({exportSubmitting: true});
  try { return await run(); }
  catch (error) { return {message: error instanceof Error ? error.message : String(error)}; }
  finally { owner.set({exportSubmitting: false}); }
}

/** Cancel/retry operate on the saved job, independently of edits made since it started. */
async function changeExportJob(command:'export.cancel'|'export.retry',jobId:string):Promise<ExportRefusal|undefined>{
  const owner=currentEditorStore(),snapshot=state.snapshot;
  if(!owner||!snapshot||!state.exportJobs.some(job=>job.id===jobId&&job.projectDir===snapshot.dir))return {message:editorText('This export is no longer available.')};
  if(state.connection!=='connected'||!state.biliupCanEdit)return {message:editorText('Editing permission is required to export.')};
  if(state.exportSubmitting)return {message:editorText('An export request is already pending.')};
  owner.set({exportSubmitting:true});
  try{
    const result=await call(command,{jobId});
    if(!result.ok)return {message:plainError(result.error)};
    if(result.value&&currentEditorStore()===owner)owner.set({exportJobId:result.value.jobId});
    return undefined;
  }catch(error){return {message:error instanceof Error?error.message:String(error)}}
  finally{owner.set({exportSubmitting:false})}
}

type AnalysisResult=Result<EditorCommands['analysis.start']['output']>;
const analysisRequests=new WeakMap<EditorStore,Map<string,{mediaId:string;task:Promise<AnalysisResult>}>>();
function startAnalysis(input:Omit<EditorCommands['analysis.start']['input'],'version'|'model'>):Promise<AnalysisResult> {
  const owner=currentEditorStore(),snapshot=owner?.getSnapshot();
  const media=snapshot?.snapshot?projectOf(snapshot.snapshot).media.find(media=>media.id===input.mediaId):undefined;
  const refuse=(message:string):Promise<AnalysisResult>=>{
    owner?.set({analysisError:message});
    return Promise.resolve({ok:false,error:{_tag:'AnalysisError',message}});
  };
  if(!owner||!snapshot||!media)return refuse(editorText('The project changed. Try the action again.'));
  if(snapshot.connection!=='connected'||!snapshot.biliupCanEdit)return refuse(editorText('当前账号没有剪辑权限'));
  const model=input.analyzers?.includes('transcript')?snapshot.transcription?.selected:undefined;
  if(input.analyzers?.includes('transcript')&&(!model||snapshot.transcriptionPending))return refuse(editorText('Wait for the speech model selection to finish.'));
  const request={...input,version:media.path,...(model?{model}:{})};
  const key=JSON.stringify([input.mediaId,media.path,model,[...(input.analyzers??['loudness'])].sort(),input.restoreJob]);
  const pending=analysisRequests.get(owner)??new Map<string,{mediaId:string;task:Promise<AnalysisResult>}>();
  analysisRequests.set(owner,pending);
  const existing=pending.get(key);if(existing)return existing.task;
  const task=call('analysis.start',request).then(result=>{
    if(!result.ok&&currentEditorStore()===owner)owner.set({analysisError:plainError(result.error)});
    return result;
  }).finally(()=>{
    pending.delete(key);
    owner.set({analysisStarting:[...new Set([...pending.values()].map(entry=>entry.mediaId))]});
  });
  pending.set(key,{mediaId:input.mediaId,task});
  owner.set({analysisError:undefined,analysisStarting:[...new Set([...pending.values()].map(entry=>entry.mediaId))]});
  return task;
}

async function cancelAnalysis(jobId:string){
  const owner=currentEditorStore(),snapshot=owner?.getSnapshot();
  if(!owner||!snapshot||snapshot.connection!=='connected'||!snapshot.biliupCanEdit||snapshot.analysisCancelling?.includes(jobId))return;
  owner.set({analysisCancelling:[...(snapshot.analysisCancelling??[]),jobId]});
  try{
    const result=await call('analysis.cancel',{jobId});
    if(!result.ok&&currentEditorStore()===owner)owner.set({analysisError:plainError(result.error),notice:{text:plainError(result.error)}});
  }finally{owner.set({analysisCancelling:owner.getSnapshot().analysisCancelling?.filter(id=>id!==jobId)})}
}

async function changeTranscription<K extends 'transcription.download'|'transcription.cancelDownload'|'transcription.selectModel'>(command:K,input:EditorCommands[K]['input']) {
  const owner=currentEditorStore();
  if(!owner||state.connection!=='connected'||!state.biliupCanEdit||state.transcriptionPending)return;
  owner.set({transcriptionPending:true,transcriptionError:undefined});
  try{
    const result=await call(command,input);
    if(!result.ok&&currentEditorStore()===owner)owner.set({transcriptionError:plainError(result.error)});
  }finally{owner.set({transcriptionPending:false})}
}

export const exportActions = {
  /** ⌘E / the Export button: the export dialog, with ffmpeg checked (it shows the fix when missing). */
  openExport() {
    if (!state.snapshot) return;
    const running=activeExport(state.exportJobs,state.snapshot.dir,state.exportJobId);
    set({ exportOpen: true,...(running?{exportJobId:running.id}:{}) });
  },
  closeExport: () => set({ exportOpen: false }),
  refreshExportJobs:()=>call('export.jobs',{}),
  /** Starts an export of the open project; returns the server's refusal (exists, no ffmpeg…) if any. */
  async startExport(input: Omit<EditorCommands['export.start']['input'], 'baseRevision'>, origin: ExportOrigin): Promise<ExportRefusal | undefined> {
    const owner=currentEditorStore();
    return submitExport(origin, async () => {
      const result = await call("export.start", {...input, baseRevision: origin.baseRevision});
      if (!result.ok) return { message: plainError(result.error) };
      if(currentEditorStore()===owner)set({ exportJobId: result.value.jobId });
      return undefined;
    });
  },
  cancelExport: (jobId: string) => changeExportJob('export.cancel',jobId),
  retryExport: (jobId: string) => changeExportJob('export.retry',jobId),
  /** Download the subtitles for the chosen range directly in the browser. */
  async exportSubtitles(input: Omit<EditorCommands['export.subtitles']['input'], 'baseRevision'>, origin: ExportOrigin): Promise<{ readonly path: string; readonly cues: number } | ExportRefusal> {
    return submitExport(origin, async () => {
      const result = await call("export.subtitles", {...input, baseRevision: origin.baseRevision});
      if (!result.ok) return { message: plainError(result.error) };
      return result.value;
    });
  },

  /** The Library's Analyze button (D66): loudness and silences of a media item, on demand. */
  analyzeMedia(mediaId: string, analyzers?: Array<'video' | 'loudness'>) {
    return startAnalysis({mediaId,...(analyzers?{analyzers}:{})});
  },
  cancelAnalysis,
  /** Transcribe a media item (the Transcript panel, D69): the analysis job, as the Library starts one. */
  transcribe(mediaId: string) {
    return startAnalysis({mediaId,analyzers:['transcript']});
  },
  retryAnalyses(mediaId:string,restoreJob:string){
    return startAnalysis({mediaId,restoreJob});
  },
  /** Find the beats of a media item (D70: the inspector's Find beats): the analysis job. */
  analyzeBeats(mediaId: string) {
    return startAnalysis({mediaId,analyzers:['beats']});
  },
  /** A cached analysis result (no job): undefined when the call failed, null when there's none yet. */
  async analysisCached(mediaId: string, analyzer: "loudness" | "transcript" | "beats" | "video", omit?: ReadonlyArray<string>): Promise<unknown> {
    const owner=currentEditorStore();
    const version=state.snapshot?projectOf(state.snapshot).media.find(media=>media.id===mediaId)?.path:undefined
    const model=analyzer==='transcript'?state.transcription?.selected:undefined
    if(!owner||!version||(analyzer==='transcript'&&!model))return undefined
    const result = await call("analysis.cached", { mediaId, analyzer, version, ...(model?{model}:{}), ...(omit ? { omit } : {}) });
    if(currentEditorStore()!==owner||(state.snapshot?projectOf(state.snapshot).media.find(media=>media.id===mediaId)?.path:undefined)!==version||(model&&state.transcription?.selected!==model))return undefined
    return result.ok ? result.value.result : undefined;
  },
  dismissAnalysisError: () => set({ analysisError: undefined }),

  /** The Setup row's speech model (D68): download, cancel, choose. */
  refreshTranscription(){return call('transcription.state',{})},
  downloadModel(model?: string) {
    return changeTranscription('transcription.download',model?{model}:{});
  },
  cancelModelDownload(model?: string) {
    return changeTranscription('transcription.cancelDownload',model?{model}:{});
  },
  selectModel(model: string) {
    return changeTranscription('transcription.selectModel',{model});
  },
};
