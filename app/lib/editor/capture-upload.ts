import {API_BASE,handleResponse} from '../api-streamer'
import {editorText} from './i18n'
export type CaptureProgress={phase:'uploading'|'processing'|'saving';loaded:number;total:number}

/** Fetch has no portable upload progress. Use the same cookie auth/error handling via XHR. */
export function uploadCapture(url:string,file:File,signal:AbortSignal,onProgress?:(progress:CaptureProgress)=>void):Promise<Response>{
  return new Promise((resolve,reject)=>{
    const xhr=new XMLHttpRequest()
    const abort=()=>xhr.abort()
    const clear=()=>signal.removeEventListener('abort',abort)
    xhr.open('POST',API_BASE+url);xhr.withCredentials=true;xhr.timeout=60*60*1000
    xhr.setRequestHeader('Content-Type','application/octet-stream')
    xhr.upload.onprogress=event=>onProgress?.({phase:'uploading',loaded:event.loaded,total:event.lengthComputable?event.total:file.size})
    xhr.upload.onload=()=>onProgress?.({phase:'processing',loaded:file.size,total:file.size})
    xhr.onload=()=>{clear();void(async()=>{const response=new Response(xhr.responseText,{status:xhr.status,statusText:xhr.statusText,headers:{'Content-Type':xhr.getResponseHeader('Content-Type')??'application/json'}});await handleResponse(response);resolve(response)})().catch(reject)}
    xhr.onerror=()=>{clear();reject(new Error(editorText('Recording upload failed. Check the connection and retry.')))}
    xhr.ontimeout=()=>{clear();reject(new Error(editorText('Recording processing timed out. The original recording was kept.')))}
    xhr.onabort=()=>{clear();reject(new Error(editorText('Recording upload was stopped. The original recording was kept.')))}
    signal.addEventListener('abort',abort,{once:true})
    if(signal.aborted){clear();reject(new Error(editorText('This project was closed.')));return}
    try{onProgress?.({phase:'uploading',loaded:0,total:file.size});xhr.send(file)}catch(error){clear();reject(error)}
  })
}
