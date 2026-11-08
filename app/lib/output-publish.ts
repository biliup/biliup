import {apiFetch,handleResponse} from './api-streamer'
export interface Publication {
 id:string;state:'uploading'|'submitting'|'succeeded'|'failed'|'unknown';task_id:string|null;bvid:string|null;error:string|null;updated_at:number
}
export interface PublishFields {
 title:string;desc:string;tags:string[];tid:number;tid_v2:number|null;copyright:number;source:string;dtime:number|null
}
export interface PublishPreview {
 draft:string|null;expires_at:number;file_name:string;duration_ms:number;template_name:string;problem:string|null;cover_error:string|null;previous:Publication|null;cover:OutputCover|null
 studio:PublishFields&{dynamic:string;only_self:boolean}
}
export interface OutputCover {id:string;name:string;source:'template'|'upload'|'frame';at_ms:number|null;width:number;height:number;url:string}
export type CoverSelection={source:'template'|'default'}|{source:'custom';id:string}
export const publicationLabels={uploading:'Uploading…',submitting:'Submitting…',succeeded:'Published',failed:'Publication failed',unknown:'Submission result unknown'} as const
export const publishable=(entry:{kind:string;job:object})=>entry.kind==='editor'||entry.kind==='render'&&!('clip_id' in entry.job&&entry.job.clip_id)
export const outputPublishUrl=(kind:string,id:number)=>'/v1/outputs/'+kind+'/'+id+'/publish'
async function post<T>(url:string,body:unknown):Promise<T>{
 const response=await apiFetch(url,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)})
 await handleResponse(response);return response.json()
}
export const previewOutput=(kind:string,id:number,template_id:number,fields:Partial<PublishFields>,cover:CoverSelection)=>post<PublishPreview>(outputPublishUrl(kind,id)+'/preview',{template_id,fields,cover})
export const submitOutput=(kind:string,id:number,draft:string,confirm_previous:boolean)=>post<Publication>(outputPublishUrl(kind,id),{draft,confirm_previous})
export const captureOutputCover=(kind:string,id:number,at_ms:number)=>post<OutputCover>(outputPublishUrl(kind,id)+'/covers',{at_ms})
export async function uploadOutputCover(kind:string,id:number,file:File):Promise<OutputCover>{
 const response=await apiFetch(outputPublishUrl(kind,id)+'/covers?name='+encodeURIComponent(file.name),{method:'PUT',headers:{'Content-Type':file.type||'application/octet-stream'},body:file})
 await handleResponse(response);return response.json()
}
