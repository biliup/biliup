import {apiFetch,fetcher,handleResponse} from '../api-streamer'
import type {SoundFolderInfo} from './contracts'
import type {FileSource} from './project'
import type {Media} from './core/timeline/types'
import {FOLDER_AUDIO_EXTENSIONS} from './core/audio/library'
import {selectBrowserFiles} from './browser-imports'

const selections=new Map<string,File[]>()
export const releaseSoundFolder=(key:string)=>selections.delete(key)
/** User gesture opens a browser directory picker; no native absolute path leaves the browser. */
export async function chooseSoundFolder():Promise<string|null>{
  const files=await selectBrowserFiles({directory:true})
  if(!files.length)return null
  const key=`browser-sounds:${crypto.randomUUID()}`;selections.set(key,files);return key
}
export class SoundLibrary{
  constructor(private project:number,private canEdit:()=>boolean,private signal:AbortSignal,private progress:(name:string,done:number,total:number)=>void){}
  private route(){return `/v1/editor/projects/${this.project}/library/folders`}
  private requireEditable(){this.signal.throwIfAborted();if(!this.canEdit())throw new Error('当前账号没有剪辑权限')}
  list(){return fetcher(this.route(),{signal:this.signal}) as Promise<SoundFolderInfo[]>}
  async remove(id:string){
    this.requireEditable()
    const response=await apiFetch(`${this.route()}/${encodeURIComponent(id)}`,{method:'DELETE',signal:this.signal});await handleResponse(response);return response.json() as Promise<SoundFolderInfo[]>
  }
  async importFolder(key:string,target?:{id:string;version:number}){
    const picked=selections.get(key);selections.delete(key)
    this.requireEditable()
    if(!picked)throw new Error('请重新选择音频文件夹')
    const files=picked.filter(file=>FOLDER_AUDIO_EXTENSIONS.includes(file.name.split('.').pop()?.toLowerCase()??''))
      .sort((a,b)=>(a.webkitRelativePath||a.name).localeCompare(b.webkitRelativePath||b.name,undefined,{numeric:true}))
    if(!files.length)throw new Error('所选文件夹没有可导入的音频')
    if(files.length>500)throw new Error('每个音频文件夹最多导入 500 个文件，请选择较小的子文件夹')
    const folders=await this.list(),existing=target?folders.find(folder=>folder.id===target.id):undefined
    if(target&&(!existing||existing.version!==target.version))throw new Error('音频文件夹已更新或移除，请刷新后重试')
    const base=files[0].webkitRelativePath.split('/')[0]||'音频文件夹'
    let name=existing?.name??base
    if(!target){let suffix=2;while(folders.some(folder=>folder.name===name))name=`${base} (${suffix++})`}
    const id=target?.id??crypto.randomUUID()
    const sounds:{assetId:string;name:string;rel:string;bytes:number}[]=[],failed:{path:string;message:string}[]=[]
    for(const [index,file] of files.entries()){
      this.requireEditable();this.progress(file.name,index,files.length)
      try{
        const response=await apiFetch(`/v1/editor/projects/${this.project}/imports?name=${encodeURIComponent(file.name)}`,{method:'POST',body:file,signal:this.signal,headers:{'Content-Type':'application/octet-stream'}})
        await handleResponse(response)
        const entry=await response.json() as {media:Media;source:FileSource}
        if(entry.media.kind!=='audio')throw new Error('文件不是音频')
        sounds.push({assetId:entry.source.assetId,name:file.name.replace(/\.[^.]+$/,''),rel:file.webkitRelativePath.split('/').slice(1).join('/')||file.name,bytes:file.size})
      }catch(error){this.signal.throwIfAborted();failed.push({path:file.name,message:error instanceof Error?error.message:String(error)})}
    }
    if(!sounds.length)throw new Error(failed[0]?.message||'没有可导入的音频')
    // Preserve the old folder when one of the replacement files failed to upload.
    if(failed.length&&target)throw new Error(`文件夹未更新：${failed.length} 个文件导入失败（${failed[0].path}：${failed[0].message}）`)
    this.requireEditable()
    const response=await apiFetch(this.route(),{method:'PUT',signal:this.signal,headers:{'Content-Type':'application/json'},body:JSON.stringify({id,name,sounds,...(target?{version:target.version}:{})})})
    if(response.status===409)throw new Error('音频文件夹已更新或移除，请刷新后重试')
    await handleResponse(response)
    return {folders:await response.json() as SoundFolderInfo[],failed}
  }
  async sound(id:string){return fetcher(`/v1/editor/projects/${this.project}/library/sounds/${encodeURIComponent(id)}/metadata`,{signal:this.signal}) as Promise<{media:Media;source:FileSource}>}
}
