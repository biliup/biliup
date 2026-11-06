import {apiFetch,fetcher,handleResponse} from '../api-streamer'
import type {AgentPlan,BrandActive,BrandKit,BrandScope,BrandSkill,BrandState,BrandImported,TemplateInfo} from './contracts'
import {selectedBrandFiles,releaseBrandFiles} from './browser-files'
import type {Project} from './core/timeline/types'
import {BUILTIN_TEMPLATES,BUILTIN_TEMPLATE_IDS,fillTemplate,templateValues} from './core/templates'
import {describeBrandFile,isBrandImport,MAX_BRAND_IMPORT_FILES} from './brand-import'
import type {BeatsResult} from './agent/analysisTypes'
import {templateProjectFacts} from './template-facts'
import {editorText} from './i18n'

type Resource<T>={id:string;scope:BrandScope;value:T}
type ResourceKind='kit'|'skill'|'active'|'template'
/** Persistent editorial metadata belongs to the server, with current-project overrides. */
export class EditorResources {
  constructor(private id:number,private canEdit:()=>boolean,private project:()=>Project,private readPlan:()=>Promise<AgentPlan|undefined>,private signal:AbortSignal){}
  private route(kind:ResourceKind){return `/v1/editor/projects/${this.id}/resources/${kind}`}
  private list<T>(kind:ResourceKind){return fetcher(this.route(kind),{signal:this.signal}) as Promise<Resource<T>[]>}
  private requireEditable(){
    this.signal.throwIfAborted()
    if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
  }
  private async write(kind:ResourceKind,id:string,scope:BrandScope,value?:unknown){
    this.requireEditable()
    await handleResponse(await apiFetch(this.route(kind),{method:value===undefined?'DELETE':'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({id,scope,...(value===undefined?{}:{value})})}))
  }
  async brand():Promise<BrandState>{return fetcher(`/v1/editor/projects/${this.id}/brand`,{signal:this.signal}) as Promise<BrandState>}
  async saveKit(kit:BrandKit){await this.write('kit',kit.id,kit.scope,kit);return this.brand()}
  async deleteKit(id:string,scope:BrandScope){await this.write('kit',id,scope);return this.brand()}
  async saveSkill(skill:BrandSkill){await this.write('skill',skill.id,skill.scope,skill);return this.brand()}
  async deleteSkill(id:string,scope:BrandScope){await this.write('skill',id,scope);return this.brand()}
  async active(active:BrandActive){await this.write('active','active','project',active);return this.brand()}
  private async sourceFiles(source:string):Promise<ReadonlyArray<File>>{
    this.signal.throwIfAborted()
    const selected=selectedBrandFiles(source);if(selected)return selected
    if(source.startsWith(`/v1/editor/projects/${this.id}/brand/`)){const response=await apiFetch(source,{signal:this.signal});await handleResponse(response);return [new File([await response.blob()],decodeURIComponent(source.split('/').pop()!))]}
    throw new Error('请选择本机文件、文件夹或输入图片、字体的 HTTPS 链接')
  }
  private async importUrl(kit:string,scope:BrandScope,url:string){
    this.requireEditable();
    const response=await apiFetch(`/v1/editor/projects/${this.id}/brand/${scope}/${encodeURIComponent(kit)}/import`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({url})});
    await handleResponse(response);return (await response.json() as {name:string}).name;
  }
  private async upload(kit:string,scope:BrandScope,file:File){
    this.requireEditable();if(file.size>200*1024*1024)throw new Error('品牌文件必须小于 200 MB')
    const response=await apiFetch(`/v1/editor/projects/${this.id}/brand/${scope}/${encodeURIComponent(kit)}/files?name=${encodeURIComponent(file.name)}`,{method:'POST',headers:{'Content-Type':'application/octet-stream'},body:file});await handleResponse(response);return (await response.json() as {name:string}).name
  }
  async addFile(kit:string,scope:BrandScope,source:string){
    try{
      this.requireEditable();
      let name:string;
      if(/^https:\/\//i.test(source.trim()))name=await this.importUrl(kit,scope,source.trim());
      else{const files=await this.sourceFiles(source);if(!files.length)throw new Error('未选择文件');name=await this.upload(kit,scope,files[0]);}
      return {name,state:await this.brand()};
    }finally{releaseBrandFiles(source)}
  }
  async importFiles(kit:string,scope:BrandScope,source:string){
    try{
      this.requireEditable();
      if(/^https:\/\//i.test(source.trim()))return {files:[describeBrandFile(await this.importUrl(kit,scope,source.trim()))],state:await this.brand()};
      const files=(await this.sourceFiles(source)).filter(file=>isBrandImport(file.name));
      if(!files.length)throw new Error('所选文件或文件夹没有支持的图片、字体文件');
      const imported:BrandImported[]=[],failed:{name:string;error:string}[]=[];
      for(const file of files.slice(0,MAX_BRAND_IMPORT_FILES)){
        this.requireEditable();
        try{const name=await this.upload(kit,scope,file);imported.push(describeBrandFile(name));}
        catch(error){failed.push({name:file.name,error:error instanceof Error?error.message:String(error)});}
      }
      return {files:imported,failed,skipped:Math.max(0,files.length-MAX_BRAND_IMPORT_FILES),state:await this.brand()};
    }finally{releaseBrandFiles(source)}
  }
  async templates():Promise<ReadonlyArray<TemplateInfo>>{return [...BUILTIN_TEMPLATES,...(await this.list<TemplateInfo>('template')).map(entry=>entry.value).filter(template=>!BUILTIN_TEMPLATE_IDS.has(template.id))]}
  async saveTemplate(template:TemplateInfo){if(BUILTIN_TEMPLATE_IDS.has(template.id))throw new Error('请复制内置模板后编辑');await this.write('template',template.id,'global',template);return this.templates()}
  async deleteTemplate(id:string){if(BUILTIN_TEMPLATE_IDS.has(id))throw new Error('不能删除内置模板');await this.write('template',id,'global');return this.templates()}
  async fill(id:string){
    const project=this.project()
    const [templates,brand,plan]=await Promise.all([this.templates(),this.brand(),this.readPlan()])
    const template=templates.find(template=>template.id===id);if(!template)throw new Error('模板不存在')
    const facts=await templateProjectFacts(project,brand,plan?.brief?.duration,async media=>{
      let response:Response
      try{
        response=await apiFetch(`/v1/editor/projects/${this.id}/analysis/media/${encodeURIComponent(media.id)}?analyzer=beats&version=${encodeURIComponent(media.path)}`,{signal:this.signal})
      }catch(error){
        this.signal.throwIfAborted()
        if(error instanceof TypeError)return undefined
        throw error
      }
      // Beat facts are optional: an unavailable recording or cache service must
      // not prevent filling the rest of the brief. Auth and invalid requests still fail.
      if(response.status===404||response.status===410||response.status>=500)return undefined
      await handleResponse(response)
      const cached=await response.json() as {result:BeatsResult|null}
      return cached.result??undefined
    })
    this.signal.throwIfAborted()
    if(this.project()!==project)throw new Error(editorText('The project changed while filling the template. Apply it again.'))
    return fillTemplate(template.body,templateValues(facts))
  }
}
