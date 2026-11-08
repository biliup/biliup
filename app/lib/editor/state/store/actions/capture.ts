// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type TakeKind, takeUploadName, type MultiLayout } from "@/app/lib/editor/contracts";
import { currentEditorStore } from "@/app/lib/editor/state/store/core";
import { plainError,currentCommandService,currentUploadSession,type FileUploader } from "@/app/lib/editor/state/store/server";
import type {CaptureProgress} from '@/app/lib/editor/capture-upload'
import {editorText} from '@/app/lib/editor/i18n'

export type TakeSaveOptions={readonly recoveryId:string;readonly placeFps:number;readonly onProgress?:(progress:CaptureProgress)=>void}
// A retry in this editor session reuses successful uploads, including parts of a multi-source take.
const uploadedTakes=new WeakMap<Blob,{dir:string;session:object;path:string}>()
const uploadPart=async(session:FileUploader,blob:Blob,kind:TakeKind,dir:string,onProgress?:TakeSaveOptions['onProgress'])=>{
  const old=uploadedTakes.get(blob)
  if(old?.dir===dir&&old.session===session)return {path:old.path}
  const ext=blob.type.includes('mp4')?'mp4':'webm'
  const result=await session(new File([blob],takeUploadName(kind,ext),{type:blob.type}),{kind,dir,onProgress})
  if('path'in result)uploadedTakes.set(blob,{dir,session,path:result.path})
  return result
}

function captureTarget(dir:string){
  const owner=currentEditorStore(),upload=currentUploadSession(),commands=currentCommandService()
  if(!owner||!upload||!commands||owner.getSnapshot().snapshot?.dir!==dir)return undefined
  const active=()=>currentEditorStore()===owner&&currentUploadSession()===upload&&currentCommandService()===commands&&owner.getSnapshot().snapshot?.dir===dir
  return {upload,commands,active}
}
const changedProject=()=>({problem:editorText("The project changed while recording, so the take wasn't added.")})

export const captureActions = {
  /** Upload a browser recording to its owning project and add it in one edit. */
  async saveTake(blob: Blob, take: { readonly kind: TakeKind; readonly name?: string; readonly device?: string; readonly placeAt?: number; readonly dir: string }&TakeSaveOptions): Promise<{ readonly saved: { readonly mediaId: string; readonly name: string; readonly seconds: number; readonly itemId?: string | undefined; readonly trackId?: string | undefined } } | { readonly problem: string }> {
    const target=captureTarget(take.dir);
    if(!target)return changedProject();
    const uploaded = await uploadPart(target.upload,blob,take.kind,take.dir,take.onProgress);
    if ("message" in uploaded) return { problem: editorText('Recording could not be uploaded: {0}',{0:uploaded.message}) };
    if(!target.active())return changedProject();
    // Converting a long camera or screen take can take a while: wait up to 10 minutes.
    take.onProgress?.({phase:'saving',loaded:blob.size,total:blob.size})
    const result = await target.commands("capture.save", { path: uploaded.path, kind: take.kind, recoveryId:take.recoveryId,placeFps:take.placeFps,...(take.name ? { name: take.name } : {}), ...(take.device ? { device: take.device } : {}), ...(take.placeAt !== undefined ? { placeAt: take.placeAt } : {}) }, 10 * 60 * 1000);
    if (!result.ok) return { problem: plainError(result.error) };
    return { saved: result.value };
  },
  /**
   * D211: a multi-source take: each part uploaded on its own, then capture.saveMulti converts them
   * into the take's folder and imports them as one edit (placed at `placeAt`, start-aligned, grouped).
   */
  async saveMultiTake(take: { readonly parts: ReadonlyArray<{ readonly blob: Blob; readonly kind: TakeKind; readonly device?: string | undefined; readonly offsetMs: number; readonly recordedMs: number }>; readonly name?: string | undefined; readonly placeAt?: number; readonly layout?: MultiLayout | undefined; readonly dir: string }&TakeSaveOptions): Promise<{ readonly saved: { readonly name: string; readonly seconds: number; readonly items: ReadonlyArray<{ readonly itemId: string }> } } | { readonly problem: string }> {
    const target=captureTarget(take.dir);
    if(!target)return changedProject();
    const parts: Array<{ path: string; kind: TakeKind; device?: string; offsetMs: number; recordedMs: number }> = [];
    const total=take.parts.reduce((sum,part)=>sum+part.blob.size,0);let completed=0
    for (const part of take.parts) {
      if(!target.active())return changedProject();
      const uploaded = await uploadPart(target.upload,part.blob,part.kind,take.dir,progress=>take.onProgress?.({...progress,loaded:completed+progress.loaded,total}));
      if ("message" in uploaded) return { problem: editorText('Recording could not be uploaded: {0}',{0:uploaded.message}) };
      completed+=part.blob.size
      parts.push({ path: uploaded.path, kind: part.kind, ...(part.device ? { device: part.device } : {}), offsetMs: Math.max(0, Math.round(part.offsetMs)), recordedMs: Math.round(part.recordedMs) });
    }
    if(!target.active())return changedProject();
    take.onProgress?.({phase:'saving',loaded:total,total})
    const result = await target.commands("capture.saveMulti", { parts,recoveryId:take.recoveryId,placeFps:take.placeFps, ...(take.name ? { name: take.name } : {}), ...(take.placeAt !== undefined ? { placeAt: take.placeAt } : {}), ...(take.layout ? { layout: take.layout } : {}) }, 10 * 60 * 1000);
    if (!result.ok) return { problem: plainError(result.error) };
    return { saved: result.value };
  },
};
