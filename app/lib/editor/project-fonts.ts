import {apiFetch,fetcher,handleResponse} from '../api-streamer'
import {editorText} from './i18n'
export interface ProjectFont {hash:string;extension:string;family:string;weight:string;style:'normal'|'italic';name:string;bytes:number}
export interface ProjectFontMetadata {family?:string;weight?:string;style?:string}
export const FONT_FILE_ACCEPT='.ttf,.otf,.woff,.woff2'
export const isFontFile=(name:string)=>/\.(ttf|otf|woff2?)$/i.test(name)
const projectId=(dir:string)=>{const id=/^biliup:(\d+)$/.exec(dir)?.[1];if(!id)throw new Error('工程尚未连接');return id}
export const readProjectFonts=(dir:string,signal?:AbortSignal)=>fetcher(`/v1/editor/projects/${projectId(dir)}/fonts`,{signal}) as Promise<ProjectFont[]>
export async function uploadProjectFont(dir:string,file:File,metadata:ProjectFontMetadata={},signal?:AbortSignal):Promise<ProjectFont[]>{
  if(file.size>2*1024*1024)throw new Error('字体文件必须小于 2 MiB')
  const params=new URLSearchParams({name:file.name,...metadata})
  const response=await apiFetch(`/v1/editor/projects/${projectId(dir)}/fonts?${params}`,{method:'POST',body:file,headers:{'Content-Type':'application/octet-stream'},signal});await handleResponse(response);return response.json()
}
export interface LocalFont {family:string;style:string;fullName:string;postscriptName:string;blob:()=>Promise<Blob>}
let installed:Promise<LocalFont[]>|undefined
export const canReadLocalFonts=()=>typeof window!=='undefined'&&typeof (window as unknown as {queryLocalFonts?:unknown}).queryLocalFonts==='function'
export function localFonts():Promise<LocalFont[]>{
  if(installed)return installed
  const query=(window as unknown as {queryLocalFonts?:()=>Promise<LocalFont[]>}).queryLocalFonts
  if(!query)return Promise.resolve([])
  installed=query.call(window).catch(error=>{installed=undefined;throw error});return installed
}
/** Read the chosen family once; the ordinary importer owns uploads, progress and partial results. */
export async function localFamilyFiles(family:string,faces:ReadonlyArray<LocalFont>,known:ReadonlyArray<ProjectFont>,current:()=>boolean){
  const selected=faces.filter(face=>face.family===family)
  if(!selected.length)throw new Error(editorText('The font files are unavailable. Import a font file.'))
  if(selected.length>32)throw new Error(editorText('This family has too many faces. Import the font files you need.'))
  const files:File[]=[],fontMetadata=new Map<File,ProjectFontMetadata>(),hashes=new Set(known.filter(font=>font.family===family).map(font=>font.hash))
  for(const face of selected){
    if(!current())return
    const blob=await face.blob()
    if(!current())return
    if(blob.size>2*1024*1024)throw new Error(editorText('Font files must be smaller than 2 MiB.'))
    const bytes=await blob.arrayBuffer(),header=new Uint8Array(bytes,0,Math.min(4,bytes.byteLength))
    const hash=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),byte=>byte.toString(16).padStart(2,'0')).join('')
    if(hashes.has(hash))continue
    const ext=String.fromCharCode(...header)==='OTTO'?'otf':'ttf'
    let stem=''
    const encoder=new TextEncoder()
    for(const char of (face.postscriptName||face.family).replace(/[^\p{L}\p{N}_. -]/gu,'_')){
      if(encoder.encode(stem+char).length>220)break
      stem+=char
    }
    const file=new File([blob],`${stem||'font'}.${ext}`)
    files.push(file);fontMetadata.set(file,{family});hashes.add(hash)
  }
  return current()?{files,fontMetadata}:undefined
}
