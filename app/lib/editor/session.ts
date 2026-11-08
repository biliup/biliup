import { fetcher } from '../api-streamer'
import { loadProject, loadRevision, saveProject, stepProjectHistory, ProjectSaveConflict, type SavedProject, type ProjectSummary, type SuggestionAcceptance } from './api'
import { documentProblems, documentFingerprint, mediaForCanvas, frameForCanvas, type EditorDocument, type FileSource, type MediaSource } from './project'
import type { SaveReview } from './save-review'
import { replacementFingerprint, replacementPlan, type PreparedReplacement } from './media-replacement'
import {prepareImportedMedia} from './media-import'
import type { Overrun } from './core/timeline/reshoot'
import { createHistory, editHistory, redoHistory, undoHistory, type EditorHistory } from './history'
import {mutate} from 'swr'
import type {SessionDetail} from '../sessions'
import type {Marker} from '../markers'
import {sessionUrl,isReadable,segmentEnd} from '../sessions'
import {suggestionsUrl,type Suggestion,type SuggestionList} from '../auto-clip'
import {planRecordingImport,type RecordingImport,type RecordingImportSelection} from './recording-import'
import {readRecordingSelection,readRecordingSuggestion,requireRecordedRange,type SavedRecordingSelection} from './recording-selection'
import {recordingRanges,recordingPoints,type RecordingRange} from './recording-ranges'
import { validateOps } from './core/timeline/validate'
import type { Canvas, Edit, Media, Op } from './core/timeline/types'
import {planImportCanvas} from './core/timeline/canvasMatch'
import {recordingProbes} from './recording-probe'
import {planPlaceMedia} from './core/timeline/arrange/place'
import {decodeSubtitleBytes,parseSubtitles,planImportSubtitles,subtitleFormatOf,writeSubtitles,type ParsedCue} from './core/text/subtitles'
import {subtitlesForExport} from './export/subtitles'
import {editorText} from './i18n'
import {activeExport} from './export/jobs'
import {measureLoudness} from './export/loudness'
import type { ProjectSnapshot, ExportJob, AnalysisJob, TranscriptionState, SetupCheck, GenerateJob, AgentState, AgentPlan } from './contracts'
import {apiFetch, handleResponse} from '../api-streamer'
import type { EditorCommands } from './commands'
import { createEditorStore } from './project-store'
import type { Result } from './state/connection'
import type { Handlers } from './commands'
import {EditorResources} from './resources'
import {planPlaceTake,planPlaceMulti} from './core/timeline/place'
import {takenIds} from './core/timeline/arrange'
import type {FileUploader,FileImporter,FileImportResult} from './state/store/server'
import type {TakeKind} from './contracts'
import {sfxSound,sfxCredit} from './core/audio/library'
import {SoundLibrary} from './sound-library'
import {planAddGraphic} from '@/app/ui/editor/preview/components'
import {isFontFile,readProjectFonts,uploadProjectFont} from './project-fonts'
import {ProjectProxies} from './proxies'
import {SourceBindings} from './source-bindings'
import {RevisionHistory} from './revision-history'
import {readAgentPage,readAgentHead,mergeAgentTurns} from './agent-history'
import {uploadCapture} from './capture-upload'
import type {AgentUi} from './contracts'
import {layout} from './core/timeline/timeline'
import {changeRange} from './state/agentChange'
import {nextPanelSeq} from './state/store/agentUi'
import {planAddLibrarySound} from './state/project'
import {libraryMediaOf} from './core/audio/library'

const captureTrackNames=():Readonly<Record<TakeKind,string>>=>({screen:editorText('Screen'),camera:editorText('Camera'),voiceover:editorText('Voiceover'),system:editorText('System audio')})

// These commands do not mutate the local timeline. Long reads and cancellation
// must remain responsive while an import or save owns the project queue.
const independentCommands=new Set<keyof EditorCommands>([
  'project.get','graphics.read','history.preview',
  'library.search','library.state','library.folders',
  'analysis.cached','analysis.jobs','analysis.cancel',
  'export.jobs','export.loudness','export.loudnessCancel','export.cancel','export.retry',
  'generate.list','generate.jobs','generate.cancel',
  'media.proxies','media.proxyCancel','media.recent',
  'transcription.state','transcription.cancelDownload','setup.check',
  'agent.subscribe','agent.cancel','agent.approve','agent.screenTake',
  // Imports manage their own shared queue entry for automatic and manual callers.
  'generate.import',
])

/** Persistence and history are owned by an opened project, never by a browser-wide RPC singleton. */
export class EditorSession {
  readonly store = createEditorStore()
  private saved?: SavedProject
  private sources=new SourceBindings()
  private history?: EditorHistory
  private readonly revisions:RevisionHistory
  private queue = Promise.resolve()
  private disposed = false
  private pending = 0
  private failed = false
  private opening?:Promise<ProjectSnapshot|undefined>
  private exportPoll?:ReturnType<typeof setTimeout>
  private exportEpoch=0
  private analysisPoll?:ReturnType<typeof setTimeout>
  private analysisEpoch=0
  private cachedAnalyses=new Map<string,AnalysisJob>()
  private modelsPoll?:ReturnType<typeof setTimeout>
  private modelsEpoch=0
  private generatePoll?:ReturnType<typeof setTimeout>
  private generatingRefresh?:Promise<void>
  private agentPoll?:ReturnType<typeof setTimeout>
  private agentRefresh?:Promise<AgentState>
  private agentEpoch=0
  private agentSyncQueued=false
  private agentUiInitialized=false
  private agentUiSeen?:string
  private agentUiRevision=0
  private shaderVersion=-1
  private resourceVersion=-1
  private fontsEpoch=0
  private async refreshFonts(){
    const epoch=++this.fontsEpoch
    try{
      const projectFonts=await readProjectFonts(`biliup:${this.id}`,this.importAbort.signal)
      if(!this.disposed&&epoch===this.fontsEpoch)this.store.set({projectFonts})
    }catch(error){if(!this.disposed&&epoch===this.fontsEpoch)throw error}
  }
  private resourceRefresh?:Promise<void>
  private refreshResources(version:number){
    if(this.resourceRefresh)return this.resourceRefresh;
    const refresh=(async()=>{
      const [brand]=await Promise.all([this.resources.brand(),this.refreshFonts()]);
      if(!this.disposed){this.store.set({brand});this.resourceVersion=version;}
    })();
    this.resourceRefresh=refresh;
    void refresh.finally(()=>{if(this.resourceRefresh===refresh)this.resourceRefresh=undefined}).catch(()=>{});
    return refresh;
  }
  private shadersRefresh?:Promise<void>
  private refreshShaders():Promise<void>{
    if(this.shadersRefresh)return this.shadersRefresh
    const load=fetcher(`/v1/editor/projects/${this.id}/shaders`,{signal:this.importAbort.signal}).then(value=>{
      const catalog=value as {revision:number;definitions:import('./core/visual/shaders').ShaderDef[]}
      if(!this.disposed&&catalog.revision>=this.shaderVersion){this.shaderVersion=catalog.revision;this.store.set({projectShaders:catalog.definitions})}
    })
    this.shadersRefresh=load
    void load.finally(()=>{if(this.shadersRefresh===load)this.shadersRefresh=undefined}).catch(()=>{})
    return load
  }
  private waitingAgentUi?:AgentUi
  private applyAgentUi(){
    const ui=this.waitingAgentUi
    if(!ui||!this.history||this.history.current.revision<ui.revision)return
    this.waitingAgentUi=undefined
    const project=this.history.current.project,{starts,duration}=layout(project)
    const range=changeRange(project.tracks.map(track=>({items:track.items.map(item=>({id:item.id,start:starts.get(item.id)??item.start??0,duration:item.duration??0}))})),ui)
    const seq=nextPanelSeq()
    this.store.set({...(ui.panel?{panelRequest:{panel:ui.panel,seq}}:{}),...(range?{agentChange:{from:Math.min(range.from,duration),to:Math.min(range.to,duration),...(ui.select&&starts.has(ui.select)?{select:ui.select}:{}),seq}}:{})})
  }
  private async applyRemoteProject(remote:SavedProject){
    if(this.disposed||!this.saved||!this.history||remote.revision<=this.saved.revision)return
    if(this.failed||this.store.getSnapshot().biliupDirty)throw new Error('工程已有新版本；本地未保存的编辑已保留，请先处理保存冲突')
    try{await this.revisions.head(false,remote.history.branchRevision!==this.saved.history.branchRevision)}catch(error){this.revisions.failed(error)}
    if(this.disposed)return
    const before=this.history.current.project
    const next=createHistory(remote.document.project)
    next.current={...next.current,revision:remote.revision}
    next.undo=[...this.history.undo,before]
    this.sources.document(remote.document)
    this.saved=remote;this.history=next;this.failed=false
    this.store.set({biliupDirty:false,biliupSaveError:undefined,biliupSaveConflict:false})
    this.publish();void this.proxies.refresh(true);void mutate(key=>typeof key==='string'&&key.startsWith('/v1/editor/projects'))
  }
  private scheduleAgentSync(){
    if(this.agentSyncQueued||this.disposed)return
    this.agentSyncQueued=true
    void this.enqueue(async()=>{
      if(this.disposed||!this.saved)return
      const agent=this.store.getSnapshot().agent
      if(agent?.projectRevision!==undefined&&agent.projectRevision>this.saved.revision){
        await this.applyRemoteProject(await loadProject(this.id))
      }
      this.applyAgentUi()
      const current=this.store.getSnapshot(),queued=current.agentQueued
      if(queued&&!queued.paused&&current.agent?.running===null&&!current.agentSending){
        if(this.failed||current.biliupDirty)throw new Error('请先保存当前编辑，再发送等待中的消息')
        this.store.set({agentQueued:undefined})
        try{await this.sendAgent(queued.request)}
        catch(error){if(!this.disposed){this.store.set({agentQueued:{...queued,paused:true}});throw error}}
      }
    }).catch(error=>{if(!this.disposed)this.store.set({agentError:error instanceof Error?error.message:String(error)})}).finally(()=>{this.agentSyncQueued=false})
  }
  private async sendAgent(input:EditorCommands['agent.send']['input']):Promise<{turnId:string}>{
    if(this.disposed||!this.saved||!this.history)throw new Error('工程尚未打开')
    if(this.failed||this.store.getSnapshot().biliupDirty)throw new Error('请先保存当前编辑，再发送给助手')
    if(this.store.getSnapshot().agentSending)throw new Error('消息正在发送')
    this.store.set({agentSending:true,agentError:undefined});this.agentEpoch++
    try{
      const result=await this.agentRequest<{turnId:string}>('',{
        prompt:input.prompt,images:input.images,access:input.access,model:input.model,provider:input.provider,effort:input.effort,
        frame:input.frame,selected:input.selected?[input.selected]:[],mentions:input.mentions,
        ranges:Object.entries(input.ranges??{}).map(([kind,range])=>({kind,...range})),
        skipStoryboard:input.skipStoryboard,baseRevision:this.saved.revision,
      })
      this.agentEpoch++
      const agent=this.store.getSnapshot().agent
      if(!this.disposed&&agent)this.store.set({agent:{...agent,running:result.turnId}})
      void this.refreshAgent().catch(()=>{})
      return result
    }catch(error){this.agentEpoch++;void this.refreshAgent().catch(()=>{});throw error}
    finally{if(!this.disposed)this.store.set({agentSending:false})}
  }
  private refreshAgent():Promise<AgentState>{
    if(this.agentRefresh)return this.agentRefresh
    clearTimeout(this.agentPoll)
    const epoch=this.agentEpoch
    const refresh=(async()=>{
      let agent=await readAgentHead(this.id,this.importAbort.signal,this.store.getSnapshot().agent)
      if(!this.disposed&&epoch===this.agentEpoch){
        if(this.agentUiInitialized&&agent.ui&&agent.ui.id!==this.agentUiSeen&&agent.ui.revision>=this.agentUiRevision)this.waitingAgentUi=agent.ui
        if(agent.ui)this.agentUiRevision=Math.max(this.agentUiRevision,agent.ui.revision)
        this.agentUiSeen=agent.ui?.id;this.agentUiInitialized=true
        const current=this.store.getSnapshot().agent
        agent={...agent,turns:mergeAgentTurns(current?.turns??[],agent.turns,agent.projectRevision,agent.running),historyCursor:current?.turns.length?current.historyCursor:agent.historyCursor}
        if(current?.plan&&current.plan.revision>(agent.plan?.revision??-1))this.store.set({agent:{...agent,plan:current.plan}})
        else this.store.set({agent})
        this.scheduleAgentSync()
        if(typeof agent.shaderVersion==='number'&&agent.shaderVersion!==this.shaderVersion)void this.refreshShaders().catch(()=>{})
        if(typeof agent.resourceVersion==='number'&&agent.resourceVersion!==this.resourceVersion)void this.refreshResources(agent.resourceVersion).catch(()=>{})
      }
      return agent
    })().catch(error=>{
      if(!this.disposed&&epoch===this.agentEpoch&&!this.store.getSnapshot().agent)this.store.set({agentError:`加载助手状态失败：${error instanceof Error?error.message:String(error)}`})
      throw error
    })
    this.agentRefresh=refresh
    void refresh.finally(()=>{
      if(this.agentRefresh===refresh)this.agentRefresh=undefined
      if(!this.disposed)this.agentPoll=setTimeout(()=>void this.refreshAgent().catch(()=>{}),epoch!==this.agentEpoch?0:this.store.getSnapshot().agent?.running?1500:10000)
    }).catch(()=>{})
    return refresh
  }
  private async agentRequest<T>(path:string,input?:unknown,method='POST'):Promise<T>{
    if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
    if(this.disposed)throw new Error('工程已关闭')
    const response=await apiFetch(`/v1/editor/projects/${this.id}/agent${path}`,{method,headers:{'Content-Type':'application/json'},...(input!==undefined?{body:JSON.stringify(input)}:{}),signal:this.importAbort.signal})
    await handleResponse(response)
    return response.json() as Promise<T>
  }
  private blockedGenerated=new Set<string>()
  private generatedImports=new Map<string,Promise<void>>()
  private importGenerated(job:string):Promise<void> {
    const pending=this.generatedImports.get(job)
    if(pending)return pending
    const result=this.enqueue(async()=>{
      try{await this.saveGenerated(job)}
      catch(error){
        if(this.store.getSnapshot().generateJobs.find(j=>j.id===job)?.phase!=='cancelled')throw error
      }
    }).finally(()=>this.generatedImports.delete(job))
    this.generatedImports.set(job,result)
    return result
  }
  private async saveGenerated(job:string) {
    if(this.disposed||!this.canEdit())throw new Error('当前工程不可编辑')
    const phase=this.store.getSnapshot().generateJobs.find(j=>j.id===job)?.phase
    if(phase==='done'||phase==='cancelled')return
    if(this.failed)throw new Error('请先保存当前编辑，再导入生成结果')
    const response=await apiFetch(`/v1/editor/projects/${this.id}/generation/${encodeURIComponent(job)}`,{signal:this.importAbort.signal})
    await handleResponse(response)
    const result=await response.json() as {media:Media;source:FileSource}
    if(this.disposed)throw new Error('工程已关闭，生成结果将在重新打开时导入')
    const matching=this.matchIncomingCanvas([result.media]),project=matching.project
    const {media,existing}=prepareImportedMedia(project,result.media,result.source.durationMs)
    const document={...this.saved!.document,sources:{...this.saved!.document.sources,[media.id]:result.source}}
    const next=existing?this.history!:editHistory(this.history!,{actor:'user',ops:[...matching.ops,{op:'add_media',media}],summary:'生成素材'})
    // Save and acknowledge the job in one transaction. Do not reset the user's undo history.
    await this.persist(next,'生成素材',[job],false,'edit',undefined,document)
    this.blockedGenerated.delete(job)
    if(!this.disposed)this.store.set({generateJobs:this.store.getSnapshot().generateJobs.map(j=>j.id===job?{...j,phase:'done',status:'已加入素材',mediaId:media.id}:j)})
  }
  private refreshGenerated():Promise<void> {
    if(this.generatingRefresh)return this.generatingRefresh
    this.generatingRefresh=this.pollGenerated().finally(()=>{this.generatingRefresh=undefined})
    return this.generatingRefresh
  }
  private async pollGenerated() {
    clearTimeout(this.generatePoll)
    if(this.disposed)return
    try {
      // Read after pending saves, so a stale poll cannot offer an acknowledged result again.
      const jobs=await this.enqueue(async()=>{
        if(this.disposed)return []
        const response=await apiFetch(`/v1/editor/projects/${this.id}/generation`,{signal:this.importAbort.signal})
        await handleResponse(response)
        const incoming=await response.json() as GenerateJob[]
        const known=new Map(this.store.getSnapshot().generateJobs.map(job=>[job.id,job]))
        const jobs=incoming.map(job=>{
          const previous=known.get(job.id)
          return previous&&(previous.phase==='done'||previous.phase==='cancelled')&&(job.phase==='running'||job.phase==='importing')?previous:job
        })
        if(!this.disposed)this.store.set({generateJobs:jobs})
        return jobs
      },false)
      if(this.disposed)return
      for(const job of jobs.filter(j=>j.phase==='importing'&&!j.agentTurnId&&!this.blockedGenerated.has(j.id))) {
        if(!this.canEdit()||this.disposed)break
        try{await this.importGenerated(job.id)}
        catch(error){
          if(this.store.getSnapshot().generateJobs.find(j=>j.id===job.id)?.phase==='cancelled')continue
          this.blockedGenerated.add(job.id)
          if(!this.disposed)this.store.set({generateError:`生成结果尚未加入素材：${error instanceof Error?error.message:String(error)}`})
        }
      }
    }catch{/* Polling retries network failures; explicit actions report errors. */}
    if(!this.disposed)this.generatePoll=setTimeout(()=>void this.refreshGenerated(),2500)
  }
  private async refreshModels(){
    clearTimeout(this.modelsPoll)
    if(this.disposed)return
    const epoch=++this.modelsEpoch
    let delay=3000
    try{
      const transcription=await fetcher('/v1/editor/transcription',{signal:this.importAbort.signal}) as TranscriptionState
      if(this.disposed||epoch!==this.modelsEpoch)return
      this.store.set({transcription,transcriptionError:undefined})
      delay=transcription.downloads.some(download=>download.phase==='downloading')?1500:15000
      return transcription
    }catch(error){
      if(!this.disposed&&epoch===this.modelsEpoch)this.store.set({transcriptionError:error instanceof Error?error.message:String(error)})
    }finally{
      if(!this.disposed&&epoch===this.modelsEpoch)this.modelsPoll=setTimeout(()=>void this.refreshModels(),delay)
    }
  }
  private importAbort = new AbortController()
  private loudnessRequest?:{id:string;abort:AbortController}
  private cancelLoudness(requestId?:string){
    const active=this.loudnessRequest
    if(!active||(requestId!==undefined&&active.id!==requestId))return
    this.loudnessRequest=undefined
    active.abort.abort()
    if(!this.disposed)this.store.set({loudnessPreview:{...this.store.getSnapshot().loudnessPreview,request:undefined}})
  }
  private captures=new Map<string,{media:Media;source:FileSource;kind?:TakeKind}>()
  private uncertainCaptures=new Map<string,number>()
  private async reconcileCapture(id:string){
    if(!/^[a-f0-9-]{36}$/.test(id))throw new Error('录制标识无效')
    const revision=this.uncertainCaptures.get(id)
    if(revision===undefined)return
    if(this.failed||this.store.getSnapshot().biliupDirty)throw new Error(editorText('Resolve the unsaved edits before retrying the recording.'))
    const remote=await loadProject(this.id)
    if(this.disposed)throw new Error('工程已关闭')
    const committed=remote.revision>revision
      ?(remote.revision===revision+1?remote.document:await loadRevision(this.id,revision+1))
      :undefined
    if(this.disposed)throw new Error('工程已关闭')
    // The submitted revision acknowledges the take even if a later edit removed it.
    // Refresh the actual head, without re-inserting an already saved and undone recording.
    await this.applyRemoteProject(remote)
    if(this.disposed)throw new Error('工程已关闭')
    this.uncertainCaptures.delete(id)
    return committed?.project.media.some(media=>media.source?.takeId===id)?committed.project:undefined
  }
  private async persistCapture(next:EditorHistory,label:string,id:string){
    if(this.failed||this.store.getSnapshot().biliupDirty)throw new Error(editorText('Resolve the unsaved edits before retrying the recording.'))
    const revision=this.saved!.revision
    try{await this.persist(next,label,undefined,true);this.uncertainCaptures.delete(id)}
    catch(error){if(!(error instanceof ProjectSaveConflict))this.uncertainCaptures.set(id,revision);throw error}
  }
  private matchIncomingCanvas(incoming:ReadonlyArray<Media>){
    const document=this.currentDocument()
    return planImportCanvas(document.project,incoming,Object.fromEntries(Object.entries(document.sources).map(([id,source])=>[id,source.durationMs])))
  }
  private captureMedia(entry:{media:Media;source:FileSource},kind:TakeKind,name:string,canvas:Canvas,recoveryId:string,device?:string):Media{
    const media:Media={...mediaForCanvas(entry.media,entry.source.durationMs,canvas),id:`capture-${recoveryId}-${kind}`,name,source:{module:'capture.browser',takeId:recoveryId,kind,device}}
    this.sources.remember(media,entry.source);return media
  }
  uploadOne:FileUploader=(file,options)=>this.enqueue(async()=>{
    try{
      if(this.disposed||options?.dir&&options.dir!==`biliup:${this.id}`)throw new Error('工程已切换，录制没有添加到其他工程')
      if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
      if(!this.saved)await this.open()
      this.store.set({uploading:{name:file.name,done:0,total:1}})
      const query=new URLSearchParams({name:file.name,...(options?.kind?{capture:options.kind}:{}),...(options?.agentId?{agent_capture:options.agentId,...(options.device?{capture_source:options.device}:{})}:{})})
      const response=await uploadCapture(`/v1/editor/projects/${this.id}/imports?${query}`,file,this.importAbort.signal,options?.onProgress)
      await handleResponse(response)
      if(options?.agentId)return await response.json() as {path:string}
      const entry=await response.json() as {media:Media;source:FileSource}
      if(this.disposed)throw new Error('工程已关闭')
      const path=`capture:${crypto.randomUUID()}`
      this.captures.set(path,{...entry,kind:options?.kind})
      return {path}
    }catch(error){return {message:error instanceof Error?error.message:String(error)}}
    finally{if(!this.disposed)this.store.set({uploading:undefined})}
  })
  private captureEntry(path:string,kind:TakeKind){
    if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
    const entry=this.captures.get(path)
    if(!entry||entry.kind!==kind)throw new Error('录制文件不属于当前工程或采集类型不匹配')
    return entry
  }
  private captureFrame(input:{placeAt?:number;placeFps:number},canvas:Canvas){
    return frameForCanvas(input.placeAt??0,input.placeFps,canvas)
  }
  private async saveTake(input:EditorCommands['capture.save']['input']):Promise<EditorCommands['capture.save']['output']>{
    const acknowledged=await this.reconcileCapture(input.recoveryId)??this.history!.current.project
    const already=acknowledged.media.find(media=>media.source?.takeId===input.recoveryId&&media.source?.kind===input.kind)
    if(already){this.captures.delete(input.path);return {mediaId:already.id,name:already.name,seconds:(already.probe?.frames??0)/acknowledged.canvas.fps}}
    const entry=this.captureEntry(input.path,input.kind),matching=this.matchIncomingCanvas([entry.media]),current=matching.project
    const name=input.name?.trim()||editorText({screen:'Screen recording',camera:'Camera recording',voiceover:'Voiceover',system:'System audio'}[input.kind])
    const media=this.captureMedia(entry,input.kind,name,current.canvas,input.recoveryId,input.device),seconds=(entry.source.durationMs??0)/1000,frames=Math.max(1,Math.floor(seconds*current.canvas.fps))
    const ops:Op[]=[...matching.ops,...(current.media.some(m=>m.id===media.id)?[]:[{op:'add_media' as const,media}])]
    const placed=input.placeAt!==undefined?planPlaceTake(current,{mediaId:media.id,frames,frame:this.captureFrame(input,current.canvas),placement:input.kind==='voiceover'||input.kind==='system'?input.kind:'video',trackName:captureTrackNames()[input.kind]},takenIds({...current,media:[...current.media,media]})):undefined
    if(placed)ops.push(...placed.ops)
    if(ops.length)await this.persistCapture(editHistory(this.history!,{actor:'user',ops,summary:editorText('Save recording')}),editorText('Save recording'),input.recoveryId)
    this.captures.delete(input.path)
    return {mediaId:media.id,name,seconds,...(placed?{itemId:placed.itemId,trackId:placed.trackId}:{})}
  }
  private async saveMultiTake(input:EditorCommands['capture.saveMulti']['input']):Promise<EditorCommands['capture.saveMulti']['output']>{
    if(!input.parts.length||input.parts.length>4||new Set(input.parts.map(p=>p.kind)).size!==input.parts.length)throw new Error('采集来源无效')
    const acknowledged=await this.reconcileCapture(input.recoveryId)??this.history!.current.project
    const existing=acknowledged.media.filter(media=>media.source?.takeId===input.recoveryId)
    if(existing.length){
      if(!input.parts.every(part=>existing.some(media=>media.source?.kind===part.kind)))throw new Error('部分录制素材已改变，请先检查工程')
      for(const part of input.parts)this.captures.delete(part.path)
      return {name:input.name||editorText('Multi-source recording'),seconds:Math.min(...existing.map(media=>(media.probe?.frames??0)/acknowledged.canvas.fps)),media:existing.map(media=>({mediaId:media.id,kind:media.source!.kind as TakeKind,name:media.name,file:media.path})),items:[]}
    }
    const matching=this.matchIncomingCanvas(input.parts.map(part=>this.captureEntry(part.path,part.kind).media))
    const current=matching.project,ops:Op[]=[...matching.ops],seen=new Set(current.media.map(m=>m.id))
    const media=input.parts.map(part=>{
      if(!Number.isFinite(part.offsetMs)||part.offsetMs<0)throw new Error('采集时间无效')
      const entry=this.captureEntry(part.path,part.kind),name=editorText({screen:'Screen recording',camera:'Camera recording',voiceover:'Voiceover',system:'System audio'}[part.kind])
      const media=this.captureMedia(entry,part.kind,name,current.canvas,input.recoveryId,part.device)
      if(!seen.has(media.id)){ops.push({op:'add_media',media});seen.add(media.id)}
      return {mediaId:media.id,kind:part.kind,name,file:media.path,frames:Math.max(1,Math.floor((entry.source.durationMs??0)*current.canvas.fps/1000)),offsetMs:part.offsetMs}
    })
    const plan=planPlaceMulti(current,{parts:media,frame:this.captureFrame(input,current.canvas),fps:current.canvas.fps,canvas:current.canvas,layout:input.layout,trackNames:captureTrackNames()},new Set([...takenIds(current),...seen]))
    if(plan.items.some(item=>item.in+item.duration>media.find(m=>m.kind===item.kind)!.frames))throw new Error('采集来源没有共同的有效时间段')
    ops.push(...plan.ops)
    const name=input.name?.trim()||editorText('Multi-source recording')
    await this.persistCapture(editHistory(this.history!,{actor:'user',ops,summary:name}),name,input.recoveryId)
    for(const part of input.parts)this.captures.delete(part.path)
    return {name,seconds:(plan.items[0]?.duration??0)/current.canvas.fps,media:media.map(({frames,offsetMs,...part})=>part),items:plan.items,group:plan.group}
  }
  private async refreshExports() {
    clearTimeout(this.exportPoll)
    if(this.disposed)return
    try{await this.readExportJobs()}catch{/* Keep the last jobs and the visible error; polling retries. */}
    if(!this.disposed){this.exportPoll=setTimeout(()=>void this.refreshExports(),1500);this.exportPoll.unref?.()}
  }
  private async readExportJobs():Promise<ReadonlyArray<ExportJob>>{
    if(this.disposed)return this.store.getSnapshot().exportJobs
    const epoch=++this.exportEpoch
    try{
      const jobs=await fetcher(`/v1/editor/projects/${this.id}/exports`,{signal:this.importAbort.signal}) as ExportJob[]
      if(this.disposed||epoch!==this.exportEpoch)return this.store.getSnapshot().exportJobs
      const state=this.store.getSnapshot()
      const resumed=!state.exportJobsLoaded&&!state.exportJobId?activeExport(jobs,`biliup:${this.id}`):undefined
      this.store.set({exportJobs:jobs,exportJobsLoaded:true,exportJobsError:undefined,...(resumed?{exportJobId:resumed.id}:{})})
      return jobs
    }catch(error){
      if(this.disposed||epoch!==this.exportEpoch)return this.store.getSnapshot().exportJobs
      this.store.set({exportJobsError:error instanceof Error?error.message:String(error)})
      throw error
    }
  }
  private async readAnalysisJobs() {
    if(this.disposed)return
    const epoch=++this.analysisEpoch
    const jobs=await fetcher(`/v1/editor/projects/${this.id}/analysis`,{signal:this.importAbort.signal}) as AnalysisJob[]
    if(!this.disposed&&epoch===this.analysisEpoch){
        for(const [key,job] of this.cachedAnalyses)if(this.history?.current.project.media.find(media=>media.id===job.mediaId)?.path!==job.version)this.cachedAnalyses.delete(key)
        const previous=this.store.getSnapshot().analysisJobs
        if(jobs.some(job=>job.stage==='model'&&!previous.some(old=>old.id===job.id&&old.stage==='model')))void this.refreshModels()
        const failed=jobs.find(job=>job.phase==='failed'&&!previous.some(old=>old.id===job.id&&old.phase==='failed'))
        this.store.set({analysisJobs:[...this.cachedAnalyses.values(),...jobs],...(failed?{analysisError:failed.error}:{} )})
    }
    return jobs
  }
  private async refreshAnalysis() {
    if(this.disposed)return
    try {await this.readAnalysisJobs()}catch{/* Explicit actions report errors; a transient polling failure retries. */}
    if(!this.disposed){this.analysisPoll=setTimeout(()=>void this.refreshAnalysis(),1500);this.analysisPoll.unref?.()}
  }
  private readonly resources:EditorResources
  private readonly soundLibrary:SoundLibrary
  private readonly proxies:ProjectProxies
  constructor(readonly id: number, private canEdit: () => boolean, private close: () => void) {
    this.revisions=new RevisionHistory(id,this.importAbort.signal)
    this.proxies=new ProjectProxies(id,this.store,canEdit)
    this.resources=new EditorResources(id,canEdit,()=>this.history!.current.project,()=>this.refreshAgent().then(agent=>agent.plan),this.importAbort.signal)
    this.soundLibrary=new SoundLibrary(id,canEdit,this.importAbort.signal,(name,done,total)=>{if(!this.disposed)this.store.set({uploading:{name,done,total}})})
  }

  private snapshot(): ProjectSnapshot {
    if (!this.saved || !this.history) throw new Error('工程尚未打开')
    return { dir: `biliup:${this.id}`, project: this.history.current.project, revision: this.history.current.revision,
      canUndo:this.store.getSnapshot().biliupDirty?this.history.undo.length>0:this.saved.history.canUndo, canRedo:this.store.getSnapshot().biliupDirty?this.history.redo.length>0:this.saved.history.canRedo }
  }
  private publish() {
    if (this.disposed) return
    this.store.set({ snapshot: this.snapshot(), history: this.revisions.state(this.saved!.revision), biliupCanEdit: this.canEdit(), biliupRevision: this.saved!.revision, biliupOrigin: this.saved!.document.origin,biliupSources:Object.fromEntries(this.history!.current.project.media.map(media=>[media.id,this.sources.get(media,this.saved!.document.sources[media.id])])) })
  }
  async open() {
    if(!this.opening)this.opening=this.load().finally(()=>{this.opening=undefined})
    return this.opening
  }
  private async load() {
    const saved = await loadProject(this.id)
    if (this.disposed) return
    this.saved = saved
    this.sources.document(saved.document)
    this.history = createHistory(saved.document.project)
    this.history.current = { ...this.history.current, revision: saved.revision }
    this.failed = false
    this.store.set({ ready: true, connection: 'connected', notice: undefined, biliupDirty: false, biliupSaveError: undefined, biliupSaveConflict: false })
    this.publish()
    void this.revisions.head(true).then(()=>{if(!this.disposed)this.publish()}).catch(error=>{if(!this.disposed){this.revisions.failed(error);this.publish()}})
    if(!this.exportPoll)void this.refreshExports()
    void this.proxies.refresh(true)
    if(!this.analysisPoll)void this.refreshAnalysis()
    void this.refreshModels()
    void this.refreshGenerated()
    void this.refreshShaders().catch(()=>{})
    if(!this.agentPoll)void this.refreshAgent().catch(()=>{})
    void this.resources.brand().then(brand=>{if(!this.disposed)this.store.set({brand})}).catch(error=>{if(!this.disposed)this.store.set({brandError:error instanceof Error?error.message:String(error)})})
    void this.refreshFonts().catch(error=>{if(!this.disposed)this.store.set({brandError:error instanceof Error?error.message:String(error)})})
    return this.snapshot()
  }
  dispose() { this.disposed = true;this.cancelLoudness();this.replacements.clear();this.importAbort.abort();this.proxies.dispose();clearTimeout(this.exportPoll);clearTimeout(this.analysisPoll);clearTimeout(this.modelsPoll);clearTimeout(this.generatePoll);clearTimeout(this.agentPoll) }
  hasPendingWork() { return this.pending > 0 || this.failed }
  private replacements=new Map<string,{receipt:PreparedReplacement;source:FileSource;attempt?:{revision:number;fingerprint:string}}>()
  private async importAsset(file:File):Promise<{media:Media;source:FileSource}>{
    const response=await apiFetch(`/v1/editor/projects/${this.id}/imports?name=${encodeURIComponent(file.name)}`,{method:'POST',body:file,signal:this.importAbort.signal,headers:{'Content-Type':'application/octet-stream'}})
    await handleResponse(response)
    return response.json()
  }
  prepareReplacement(target:Pick<Media,'id'|'path'|'kind'>,file:File):Promise<PreparedReplacement>{
    return this.enqueue(async()=>{
      if(this.disposed||!this.canEdit()||!this.history)throw new Error('当前工程不可编辑')
      if(this.failed||this.store.getSnapshot().biliupDirty)throw new Error(editorText('Resolve the unsaved edits before replacing media.'))
      const current=this.history.current.project.media.find(media=>media.id===target.id)
      if(!current||current.path!==target.path)throw new Error(editorText('The source media changed. Choose the replacement again.'))
      this.store.set({uploading:{name:file.name,done:0,total:1}})
      try{
        const entry=await this.importAsset(file)
        if(this.disposed)throw new Error('工程已关闭')
        if(entry.media.kind!==current.kind)throw new Error(editorText('The replacement must have the same media type.'))
        const receipt:PreparedReplacement={token:crypto.randomUUID(),mediaId:target.id,originalPath:target.path,media:entry.media,durationMs:entry.source.durationMs}
        this.replacements.set(receipt.token,{receipt,source:entry.source})
        return receipt
      }finally{if(!this.disposed)this.store.set({uploading:undefined})}
    })
  }
  discardReplacement(token:string){this.replacements.delete(token)}
  replacementPending(token:string){return !!this.replacements.get(token)?.attempt}
  private async confirmReplacement(token:string):Promise<boolean>{
    const prepared=this.replacements.get(token),attempt=prepared?.attempt
    if(!prepared||!attempt)return false
    const remote=await loadProject(this.id)
    if(this.disposed)throw new Error('工程已关闭')
    const committed=remote.revision>attempt.revision
      ?(remote.revision===attempt.revision+1?remote.document:await loadRevision(this.id,attempt.revision+1))
      :undefined
    if(this.disposed)throw new Error('工程已关闭')
    const accepted=!!committed&&replacementFingerprint(committed)===attempt.fingerprint
    await this.applyRemoteProject(remote)
    if(this.disposed)throw new Error('工程已关闭')
    delete prepared.attempt
    if(accepted)this.replacements.delete(token)
    return accepted
  }
  replaceFile(token:string,overrun:Overrun,baseRevision:number):Promise<void>{
    return this.enqueue(async()=>{
      if(this.disposed||!this.canEdit()||!this.history)throw new Error('当前工程不可编辑')
      if(this.failed||this.store.getSnapshot().biliupDirty)throw new Error(editorText('Resolve the unsaved edits before replacing media.'))
      const prepared=this.replacements.get(token)
      if(!prepared)throw new Error(editorText('Choose a replacement file first.'))
      if(prepared.attempt){
        if(await this.confirmReplacement(token))return
        throw new Error(editorText('The replacement was not saved. Review it and try again.'))
      }
      if(this.history.current.revision!==baseRevision)throw new Error(editorText('The timeline changed. Review the replacement again.'))
      const plan=replacementPlan(this.history.current.project,prepared.receipt,overrun)
      const next=editHistory(this.history,{actor:'user',baseRevision,ops:plan.ops,summary:plan.summary})
      const media=next.current.project.media.find(value=>value.id===prepared.receipt.mediaId)!
      this.sources.remember(media,prepared.source)
      const before=this.currentDocument()
      const document={...before,project:next.current.project,sources:{...before.sources,[media.id]:prepared.source}}
      prepared.attempt={revision:baseRevision,fingerprint:replacementFingerprint(document)}
      try{await this.persist(next,plan.summary,undefined,true,'edit',undefined,document)}catch(error){
        if(error instanceof ProjectSaveConflict){delete prepared.attempt;throw error}
        // A dropped response may follow a successful save. Only the exact submitted revision
        // proves success; a later edit or undo is left intact when that receipt is acknowledged.
        try{if(await this.confirmReplacement(token))return}catch{/* Keep the receipt for Check save. */}
        throw error
      }
      this.replacements.delete(token)
    })
  }
  /** The opening that received the files owns the upload, report and one undoable edit. */
  importFiles:FileImporter = (files,{drop,fontMetadata}={}) => this.enqueue(async():Promise<FileImportResult|undefined>=>{
    if(this.disposed)return
    if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
    if(!this.saved)await this.open()
    const failed:{path:string;message:string}[]=[],skipped:{path:string;reason:string}[]=[]
    const fonts=new Set<string>()
    const imported:{media:Media;source:FileSource}[]=[]
    const graphics:{src:string;name:string}[]=[]
    const subtitles:{name:string;cues:ParsedCue[];skipped:number}[]=[]
    const warnings:NonNullable<FileImportResult['warnings']>=[]
    this.store.set({importReport:undefined})
    try {
      for(const [index,file] of files.entries()){
        if(this.disposed)return
        this.store.set({uploading:{name:file.name,done:index,total:files.length}})
        try {
          if(isFontFile(file.name)){
            const known=new Set((this.store.getSnapshot().projectFonts??[]).map(font=>font.hash))
            const projectFonts=await uploadProjectFont(`biliup:${this.id}`,file,fontMetadata?.get(file),this.importAbort.signal)
            this.fontsEpoch++
            for(const font of projectFonts)if(font.name===file.name||!known.has(font.hash))fonts.add(font.family)
            if(!this.disposed)this.store.set({projectFonts});continue
          }
          if(/\.html?$/i.test(file.name)){
            if(file.size>256*1024)throw new Error('动态图形不能超过 256 KiB')
            graphics.push(await this.writeGraphic({name:file.name,html:await file.text()}));continue
          }
          if(subtitleFormatOf(file.name)){
            if(file.size>5*1024*1024)throw new Error(editorText('Subtitle files must be smaller than 5 MB.'))
            const parsed=parseSubtitles(decodeSubtitleBytes(new Uint8Array(await file.arrayBuffer())),file.name)
            if('problem' in parsed)throw new Error(editorText(parsed.problem,parsed.values))
            subtitles.push({name:file.name,cues:parsed.cues,skipped:parsed.skipped})
            continue
          }
          imported.push(await this.importAsset(file))
        }catch(error){if(this.disposed)return;failed.push({path:file.name,message:error instanceof Error?error.message:String(error)})}
      }
      if(this.disposed)return
      const matching=this.matchIncomingCanvas(imported.map(entry=>entry.media))
      const current=matching.project,media=new Map(current.media.map(item=>[item.id,item])),toPlace:Media[]=[]
      const placement=drop?{...drop,frame:frameForCanvas(drop.frame,drop.fps,current.canvas)}:undefined
      for(const entry of imported){
        const prepared=prepareImportedMedia({...current,media:[...media.values()]},entry.media,entry.source.durationMs)
        if(prepared.existing)skipped.push({path:entry.media.name,reason:'Already in the project.'})
        else{
          media.set(prepared.media.id,prepared.media)
          this.sources.remember(prepared.media,entry.source)
        }
        toPlace.push(prepared.media)
      }
      const next={...current,media:[...media.values()]},ops:Op[]=[...matching.ops]
      const existing = new Set(current.media.map(item=>item.id))
      ops.push(...next.media.filter(item=>!existing.has(item.id)).map(item=>({op:'add_media' as const,media:item})))
      if(placement&&toPlace.length){
        const taken=new Set([...next.media.map(m=>m.id),...next.tracks.flatMap(t=>[t.id,...t.items.map(i=>i.id)])])
        const plan=planPlaceMedia(next,toPlace,placement,taken)
        if('problem' in plan)failed.push({path:'时间轴',message:plan.problem})
        else ops.push(...plan.ops)
      }
      let nextHistory=ops.length?editHistory(this.history!,{actor:'user',ops,summary:'导入素材'}):this.history!
      for(const graphic of graphics){
        const plan=planAddGraphic(nextHistory.current.project,{...graphic,props:{},idBase:'graphic'},placement?.frame??0)
        if('problem' in plan){failed.push({path:graphic.name,message:plan.problem});continue}
        const problems=validateOps(plan.ops);if(problems.length){failed.push({path:graphic.name,message:problems.map(p=>p.message).join('；')});continue}
        nextHistory=editHistory(nextHistory,{actor:'user',ops:plan.ops as ReadonlyArray<Op>,summary:'导入动态图形'})
      }
      for(const subtitle of subtitles){
        const plan=planImportSubtitles(nextHistory.current.project,subtitle.cues,subtitle.name,placement?.frame??0)
        if('problem' in plan){failed.push({path:subtitle.name,message:editorText(plan.problem,plan.values)});continue}
        const subtitleOps=plan.ops.map(op=>op.op==='add_track'?{...op,track:{...op.track,name:editorText('Subtitles')+' · '+subtitle.name}}:op)
        nextHistory=editHistory(nextHistory,{actor:'user',ops:subtitleOps,summary:editorText('Import subtitles')})
        const changes=[
          subtitle.skipped?editorText('Skipped {0} cues with empty text or invalid duration.',{0:subtitle.skipped}):'',
          plan.skipped?editorText('Skipped {0} cues after the end of the edit.',{0:plan.skipped}):'',
          plan.clipped?editorText('Shortened {0} cues at the end of the edit.',{0:plan.clipped}):'',
        ].filter(Boolean)
        if(changes.length)warnings.push({path:subtitle.name,message:changes.join(' ')})
      }
      if(nextHistory!==this.history){
        // Batch imports remain one undo step even when they contain media and several subtitle files.
        nextHistory={...nextHistory,undo:[...this.history!.undo,this.history!.current.project],redo:[]}
        await this.persist(nextHistory,'导入素材')
      }
      const references:FileImportResult['references']=toPlace.map(media=>({kind:'media',id:media.id}))
      const tracks=new Set(current.tracks.map(track=>track.id))
      const items=new Set(current.tracks.flatMap(track=>track.items.map(item=>item.id)))
      for(const track of nextHistory.current.project.tracks){
        if(!tracks.has(track.id))references.push({kind:'track',id:track.id})
        else for(const item of track.items)if(!items.has(item.id))references.push({kind:'item',id:item.id})
      }
      return {references,fonts:[...fonts],failed,skipped,warnings}
    }finally{if(!this.disposed)this.store.set({uploading:undefined,importReport:failed.length||skipped.length||warnings.length?{failed,skipped,warnings}:undefined})}
  }).catch(error=>{
    const failed=[{path:'导入素材',message:error instanceof Error?error.message:String(error)}]
    if(!this.disposed)this.store.set({importReport:{failed,skipped:[]}})
    return {references:[],fonts:[],failed,skipped:[]}
  })
  async importRecordings(selection:ReadonlyArray<RecordingImportSelection>) {
    return this.enqueue(async()=>{
      if(this.disposed||!this.canEdit())throw new Error(editorText('This project cannot be edited.'))
      const groups=new Map<number,Set<number>>()
      for(const entry of selection){
        if(!Number.isSafeInteger(entry.sessionId)||entry.sessionId<=0||entry.segmentIds.some(id=>!Number.isSafeInteger(id)||id<=0))throw new Error(editorText('Some selected recordings are no longer available.'))
        const ids=groups.get(entry.sessionId)??new Set<number>()
        for(const id of entry.segmentIds)ids.add(id)
        if(ids.size)groups.set(entry.sessionId,ids)
      }
      if(!groups.size)return this.snapshot()
      const inputs:RecordingImport[]=[]
      for(const [id,ids] of groups){
        const detail=await fetcher(sessionUrl(id),{signal:this.importAbort.signal}) as SessionDetail
        const segments=detail.segments.filter(segment=>ids.has(segment.id)&&isReadable(segment)&&segmentEnd(segment)>segment.start_ms)
        if(segments.length!==ids.size)throw new Error(editorText('Some selected recordings are no longer available.'))
        inputs.push({detail:{...detail,segments}})
      }
      await this.saveRecordingImports(inputs,editorText('Add recordings'))
      return this.snapshot()
    })
  }

  private async importRecordingNow(detail:SessionDetail,markers:Marker[]=[],range?:{inMs:number;outMs:number},title?:string){
    return this.saveRecordingImports([{detail,markers,range,title}],title??editorText(range?'Add recording range':'Add recordings'))
  }

  private async saveRecordingImports(inputs:ReadonlyArray<RecordingImport>,summary:string){
    if(this.disposed||!this.canEdit())throw new Error('当前工程不可编辑')
    if(!this.saved)await this.open()
    if(this.failed||this.store.getSnapshot().biliupDirty)throw new Error(editorText('Save your changes before adding a recording.'))
    const current=this.history!.current.project,prepared:RecordingImport[]=[]
    for(const input of inputs)prepared.push({...input,probes:await recordingProbes(input.detail,input.range,this.importAbort.signal)})
    if(this.disposed)throw new Error('工程已关闭')
    const plan=planRecordingImport(this.history!,this.currentDocument(),prepared,summary)
    const addedMedia=plan.document.project.media.filter(media=>!current.media.some(previous=>previous.id===media.id&&previous.path===media.path))
    this.sources.document(plan.document)
    try{await this.persist(plan.history,summary,undefined,true,'edit',undefined,plan.document)}catch(error){
      if(this.disposed||(!plan.items.length&&!addedMedia.length))throw error
      // Resolve a lost save response using the ordinary saved items/media, without another import.
      try{
        const remote=await loadProject(this.id)
        if(this.disposed)throw error
        await this.applyRemoteProject(remote)
        const present=new Set(remote.document.project.tracks.flatMap(track=>track.items.map(item=>item.id)))
        if(!plan.items.every(id=>present.has(id))||!addedMedia.every(media=>remote.document.project.media.some(saved=>saved.id===media.id&&saved.path===media.path)))throw error
      }catch{throw error}
    }
    const savedProject=this.history!.current.project,positions=layout(savedProject).starts
    const added=savedProject.tracks.flatMap(track=>track.items).filter(item=>plan.items.includes(item.id))
    return {items:plan.items,from:added.length?Math.min(...added.map(item=>positions.get(item.id)??0)):plan.start,
      to:added.length?Math.max(...added.map(item=>(positions.get(item.id)??0)+item.duration)):plan.end}
  }

  addRecordingSelection(selection:SavedRecordingSelection){
    return this.enqueue(async()=>{
      if(this.disposed||!this.canEdit())throw new Error(editorText('This project cannot be edited.'))
      const input=await readRecordingSelection(selection,this.importAbort.signal),document=this.currentDocument()
      if(!Object.values(document.sources).some(source=>source.kind==='recording'&&(source.sessionId===input.range.sessionId||source.viewSessionId===input.range.sessionId)))throw new Error(editorText('This recording is no longer in the project.'))
      return this.importRecordingNow(input.detail,input.markers,input.range,input.title)
    })
  }

  /** Incoming live actions share this project's queue and never create a second session over its draft. */
  openRecordingAt(detail:SessionDetail,markers:Marker[],atMs:number,range?:RecordingRange){
    return this.enqueue(async()=>{
      if(this.disposed||!this.canEdit())throw new Error(editorText('This project cannot be edited.'))
      if(!this.saved)await this.open()
      let document=this.currentDocument()
      if(range){
        let match=recordingRanges(document.project,document.sources,range,detail)[0]
        if(!match){
          requireRecordedRange(detail,range)
          match=await this.importRecordingNow(detail,markers,range)
        }
        const fps=this.history!.current.project.canvas.fps
        return {at:Math.round(match.from*1000/fps),end:Math.round(match.to*1000/fps)}
      }
      let point:ReturnType<typeof recordingPoints>[number]|undefined=recordingPoints(document.project,document.sources,detail.id,atMs,detail)[0]
      if(!point){
        const segment=detail.segments.find(segment=>isReadable(segment)&&segment.start_ms<=atMs&&segmentEnd(segment)>=atMs&&segmentEnd(segment)>segment.start_ms)
        if(!segment)throw new Error(editorText('No recorded footage is available at this position.'))
        // Opening a removed/later point adds its recording file, not another copy of the entire broadcast.
        const imported=await this.importRecordingNow(detail,markers,{inMs:segment.start_ms,outMs:segmentEnd(segment)})
        document=this.currentDocument()
        point=recordingPoints(document.project,document.sources,detail.id,atMs,detail).find(point=>point.frame>=imported.from)
        if(!point)throw new Error(editorText('No recorded footage is available at this position.'))
      }
      return {at:Math.round(point.frame*1000/document.project.canvas.fps)}
    })
  }

  acceptSuggestion(suggestion:Suggestion){
    return this.enqueue(async()=>{
      if(this.disposed||!this.canEdit()||!this.saved||!this.history)throw new Error(editorText('This project cannot be edited.'))
      if(this.failed||this.store.getSnapshot().biliupDirty)throw new Error(editorText('Save your changes before adding a recording.'))
      const sessionId=suggestion.view_session_id??suggestion.session_id
      try{
        const {detail,range,suggestion:currentSuggestion}=await readRecordingSuggestion(suggestion,this.importAbort.signal)
        const document=this.currentDocument()
        if(!Object.values(document.sources).some(source=>source.kind==='recording'&&(source.sessionId===sessionId||source.viewSessionId===sessionId)))throw new Error(editorText('This recording is no longer in the project.'))
        const probes=await recordingProbes(detail,range,this.importAbort.signal)
        if(this.disposed||!this.canEdit())throw new Error(editorText('This project cannot be edited.'))
        const label=editorText('Use suggestion: {0}',{0:currentSuggestion.title})
        const plan=planRecordingImport(this.history,document,[{detail,range,title:currentSuggestion.title,probes}],label)
        this.sources.document(plan.document)
        try{
          await this.persist(plan.history,label,undefined,true,'edit',{id:suggestion.id,items:plan.items},plan.document)
        }catch(error){
          // A lost response must not append the same suggestion twice. Read its normal saved items.
          try{
            const remote=await loadProject(this.id)
            await this.applyRemoteProject(remote)
            const list=await fetcher(suggestionsUrl(sessionId),{signal:this.importAbort.signal}) as SuggestionList
            const receipt=list.editor_uses.find(use=>use.suggestionId===suggestion.id&&use.projectId===this.id)
            const items=new Set(remote.document.project.tracks.flatMap(track=>track.items.map(item=>item.id)))
            if(!receipt||!plan.items.every(id=>items.has(id)&&receipt.items.includes(id)))throw error
          }catch{throw error}
        }
        const current=this.history!.current.project,positions=layout(current).starts
        const clips=current.tracks.flatMap(track=>track.items).filter(item=>plan.items.includes(item.id))
        const from=Math.min(...clips.map(item=>positions.get(item.id)??0)),to=Math.max(...clips.map(item=>(positions.get(item.id)??0)+item.duration))
        return {items:plan.items,from,to,fps:current.canvas.fps}
      }finally{
        void mutate(suggestionsUrl(sessionId));if(sessionId!==suggestion.session_id)void mutate(suggestionsUrl(suggestion.session_id))
      }
    })
  }

  private async stepHistory(action:'undo'|'redo'|'jump',target?:number,baseRevision?:number):Promise<ProjectSnapshot>{
    if(!this.saved||!this.history||this.disposed)throw new Error('工程尚未打开')
    if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
    if(baseRevision!==undefined&&baseRevision!==this.history.current.revision)throw new Error(editorText('The project changed. Try the action again.'))
    if(this.failed||this.store.getSnapshot().biliupDirty){
      // Keep the failed-save recovery path: unsaved local edits still have their in-memory undo.
      if(action==='undo')return this.persist(undoHistory(this.history),'撤销')
      if(action==='redo')return this.persist(redoHistory(this.history),'重做')
      await this.persist(this.history,'保存编辑')
      if(this.failed||this.store.getSnapshot().biliupDirty)throw new Error(editorText('Save or resolve the current edits before restoring a version.'))
    }
    this.store.set({biliupSaving:true})
    this.agentEpoch++
    try{
      const saved=await stepProjectHistory(this.saved,action,target)
      await this.applyRemoteProject(saved)
      this.waitingAgentUi=undefined
      if(!this.disposed)this.store.set({agentChange:undefined})
      return this.snapshot()
    }finally{this.agentEpoch++;if(!this.disposed){this.store.set({biliupSaving:false});void this.refreshAgent().catch(()=>{})}}
  }
  private async persist(next: EditorHistory, label: string, generated?:ReadonlyArray<string>,atomic=false,kind:'edit'|'jump'='edit',suggestion?:SuggestionAcceptance,prepared?:EditorDocument) {
    if (!this.saved || !this.history) throw new Error('工程尚未打开')
    if (!this.canEdit()) throw new Error('当前账号没有剪辑权限')
    const document = { ...(prepared??this.saved.document), project: next.current.project,
      sources:Object.fromEntries(next.current.project.media.map(media=>[media.id,this.sources.get(media,(prepared??this.saved!.document).sources[media.id])])) }
    const problems = documentProblems(document)
    if (problems.length) throw new Error(problems.map(problem => problem.message).join('；'))
    // A failed save keeps the complete local timeline and undo stack available for retry.
    atomic=atomic||!!generated?.length||!!suggestion
    if(!atomic){
      this.history = next;this.store.set({biliupDirty:true});this.publish()
      // Keep editing the same local draft while a known conflict awaits an explicit choice.
      if(this.store.getSnapshot().biliupSaveConflict)return this.snapshot()
    }
    this.store.set({ biliupSaving: true, ...(!atomic?{biliupDirty: true, biliupSaveError: undefined}:{}) })
    try {
      const saved = await saveProject(document, this.saved, generated,{label,kind},suggestion)
      this.sources.document(saved.document)
      this.history={...next,current:{project:saved.document.project,revision:saved.revision,log:[]}}
      this.saved = saved
      void mutate(key=>typeof key==='string'&&key.startsWith('/v1/editor/projects'))
      this.failed = false
      this.revisions.record({ revision: saved.revision, at: new Date(saved.updated_at).toISOString(), kind, author: 'user', label, lines: [], current: true })
      this.store.set({ biliupDirty: false, biliupSaveError: undefined, biliupSaveConflict: false, notice: undefined })
      this.publish()
      void this.proxies.refresh(true)
      return this.snapshot()
    } catch (error) {
      if(atomic){
        // A rejected atomic import did not change the local timeline. Refresh its base
        // so retry can replan the retained recording/generation against the current head.
        if(error instanceof ProjectSaveConflict&&!this.failed&&!this.store.getSnapshot().biliupDirty){
          try{await this.applyRemoteProject(await loadProject(this.id))}catch{/* Keep the original failure and the recoverable import. */}
        }
        throw error
      }
      this.failed = true
      const message = error instanceof Error ? error.message : String(error)
      this.store.set({ biliupSaveError: message, biliupSaveConflict: error instanceof ProjectSaveConflict, notice: { text: `保存失败，本地编辑已保留：${message}` } })
      throw error
    } finally { this.store.set({ biliupSaving: false }) }
  }
  async retrySave() {
    if (!this.history || this.pending) return
    return this.enqueue(() => this.safe(() => this.persist(this.history!, '重试保存')))
  }
  private currentDocument(): EditorDocument {
    if(this.disposed||!this.saved||!this.history)throw new Error('工程已关闭')
    return {...this.saved.document,project:this.history.current.project,
      sources:Object.fromEntries(this.history.current.project.media.map(media=>[media.id,this.sources.get(media,this.saved!.document.sources[media.id])]))}
  }
  reviewSave(): Promise<SaveReview> {
    return this.enqueue(async()=>{
      const remote=await loadProject(this.id)
      const local=structuredClone(this.currentDocument())
      return {projectId:this.id,localRevision:this.history!.current.revision,localFingerprint:documentFingerprint(local),local,remote}
    })
  }
  resolveSave(review:SaveReview,choice:'local'|'server'):Promise<void> {
    return this.enqueue(async()=>{
      if(this.disposed||!this.saved||!this.history||review.projectId!==this.id)throw new Error('工程已关闭')
      if(choice==='local'&&!this.canEdit())throw new Error('当前账号没有剪辑权限')
      const local=this.currentDocument()
      if(this.history.current.revision!==review.localRevision||documentFingerprint(local)!==review.localFingerprint)throw new Error(editorText('Local edits have changed. Refresh the comparison.'))
      this.store.set({biliupSaving:true})
      this.agentEpoch++
      try{
        const remote=await loadProject(this.id)
        if(this.disposed)throw new Error('工程已关闭')
        if(remote.revision!==review.remote.revision)throw new ProjectSaveConflict(editorText('The server version has changed. Refresh the comparison.'))
        // Keeping local changes creates a normal revision. The reviewed server version
        // remains in History and is exactly what the next Undo restores.
        const saved=choice==='server'||documentFingerprint(local)===documentFingerprint(remote.document)
          ?remote
          :await saveProject(local,remote,undefined,{label:editorText('Resolve save conflict'),kind:'edit'})
        if(this.disposed)return
        const history=createHistory(saved.document.project)
        history.current={...history.current,revision:saved.revision}
        this.saved=saved;this.history=history;this.sources.document(saved.document);this.failed=false
        this.waitingAgentUi=undefined
        this.store.set({biliupDirty:false,biliupSaveError:undefined,biliupSaveConflict:false,notice:undefined,agentError:undefined,agentChange:undefined,maskEditing:undefined})
        try{await this.revisions.head(true)}catch(error){this.revisions.failed(error)}
        this.publish();void this.proxies.refresh(true)
        void mutate(key=>typeof key==='string'&&key.startsWith('/v1/editor/projects'))
      }catch(error){
        if(!this.disposed){
          const message=error instanceof Error?error.message:String(error)
          this.failed=true
          this.store.set({biliupSaveError:message,...(error instanceof ProjectSaveConflict?{biliupSaveConflict:true}:{})})
        }
        throw error
      }finally{
        this.agentEpoch++
        if(!this.disposed){this.store.set({biliupSaving:false});void this.refreshAgent().catch(()=>{})}
      }
    })
  }
  private async safe<T>(work: () => Promise<T>): Promise<Result<T>> {
    try { return { ok: true, value: await work() } }
    catch (error) { return { ok: false, error: { _tag: error instanceof TypeError ? 'NetworkError' : 'EditRefused', message: error instanceof Error ? error.message : String(error) } } }
  }
  private handlers: Handlers = {
    'graphics.write':input=>this.safe(()=>this.writeGraphic(input)),
    'graphics.read':input=>this.safe(async()=>{if(!/^graphics\/[A-Za-z0-9][A-Za-z0-9._-]{0,80}\.html$/.test(input.src))throw new Error('图形路径无效');return await fetcher(`/v1/editor/projects/${this.id}/graphics/${encodeURIComponent(input.src.slice(9))}?raw=true`) as EditorCommands['graphics.read']['output']}),
    'generate.list':()=>this.safe(async()=>await fetcher('/v1/editor/generators') as EditorCommands['generate.list']['output']),
    'generate.jobs':()=>this.safe(async()=>await fetcher(`/v1/editor/projects/${this.id}/generation`) as GenerateJob[]),
    'generate.start':input=>this.safe(async()=>{
      if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
      const response=await apiFetch(`/v1/editor/projects/${this.id}/generation`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(input)})
      await handleResponse(response);const result=await response.json();void this.refreshGenerated();return result as {jobId:string}
    }),
    'generate.cancel':input=>this.safe(async()=>{
      if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
      await handleResponse(await apiFetch(`/v1/editor/projects/${this.id}/generation/${encodeURIComponent(input.jobId)}`,{method:'DELETE'}))
      if(!this.disposed)this.store.set({generateJobs:this.store.getSnapshot().generateJobs.map(job=>job.id===input.jobId?{...job,phase:'cancelled',status:'已取消'}:job)})
      void this.refreshGenerated()
    }),
    'generate.import':input=>this.safe(async()=>{await this.importGenerated(input.jobId);void this.refreshGenerated()}),
    'capture.save':input=>this.safe(()=>this.saveTake(input)),
    'keys.list':()=>this.safe(async()=>await fetcher('/v1/editor/keys') as EditorCommands['keys.list']['output']),
    'keys.set':input=>this.safe(async()=>{const response=await apiFetch(`/v1/editor/keys/${encodeURIComponent(input.provider)}`,{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify({key:input.key})});await handleResponse(response);return response.json() as Promise<EditorCommands['keys.set']['output']>}),
    'keys.remove':input=>this.safe(async()=>{const response=await apiFetch(`/v1/editor/keys/${encodeURIComponent(input.provider)}`,{method:'DELETE'});await handleResponse(response);return response.json() as Promise<EditorCommands['keys.remove']['output']>}),
    'capture.saveMulti':input=>this.safe(()=>this.saveMultiTake(input)),
    'library.folders':()=>this.safe(()=>this.soundLibrary.list()),
    'library.state':()=>this.safe(async()=>await fetcher('/v1/editor/library/state') as EditorCommands['library.state']['output']),
    'library.search':input=>this.safe(async()=>{const response=await apiFetch('/v1/editor/library/search',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(input),signal:this.importAbort.signal});await handleResponse(response);return response.json() as Promise<EditorCommands['library.search']['output']>}),
    'library.folderRemove':input=>this.safe(()=>this.soundLibrary.remove(input.id)),
    'library.folderAdd':input=>this.safe(async()=>{
      try{
        const result=await this.soundLibrary.importFolder(input.selection,input.target)
        if(!this.disposed)this.store.set({importReport:result.failed.length?{failed:result.failed,skipped:[]}:undefined});return result.folders
      }
      finally{if(!this.disposed)this.store.set({uploading:undefined})}
    }),
    'library.add':input=>this.safe(async()=>{
      if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
      if(!this.saved||!this.history||this.disposed)throw new Error('工程已关闭')
      const frame=frameForCanvas(input.frame,input.fps,this.history.current.project.canvas)
      let media=libraryMediaOf(this.history.current.project,input.id)
      if(!media){
        const response=await apiFetch(`/v1/editor/projects/${this.id}/library/items/${encodeURIComponent(input.id)}`,{method:'POST',signal:this.importAbort.signal})
        await handleResponse(response)
        const entry=await response.json() as {media:Media;source:FileSource}
        if(this.disposed)throw new Error('工程已关闭')
        const prepared=prepareImportedMedia(this.history.current.project,entry.media,entry.source.durationMs)
        media={...prepared.media,source:prepared.media.source??entry.media.source}
        if(!prepared.existing)this.sources.remember(media,entry.source)
      }
      const plan=planAddLibrarySound(this.history.current.project,media,input.music,frame)
      if('problem' in plan)throw new Error(editorText(plan.problem))
      const problems=validateOps(plan.ops);if(problems.length)throw new Error(problems.map(problem=>problem.message).join('；'))
      await this.persist(editHistory(this.history,{actor:'user',ops:plan.ops as ReadonlyArray<Op>,summary:plan.summary}),plan.summary)
      return {itemId:plan.itemId,frame:plan.show??plan.start}
    }),
    'analysis.start':input=>this.safe(async()=>{
      if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
      if(this.history?.current.project.media.find(media=>media.id===input.mediaId)?.path!==input.version)throw new Error(editorText('The project changed. Try the action again.'))
      const response=await apiFetch(`/v1/editor/projects/${this.id}/analysis`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(input),signal:this.importAbort.signal})
      await handleResponse(response)
      const result=await response.json() as EditorCommands['analysis.start']['output']
      if(!this.disposed){
        // The accepted response owns these jobs even if the following status read fails.
        // Cache hits also notify consumers to reread a previously absent result.
        this.analysisEpoch++
        const jobs=this.store.getSnapshot().analysisJobs
        const accepted:AnalysisJob[]=result.map(started=>{
          const known=jobs.find(job=>job.id===started.jobId)
          if(known)return {...known,mediaIds:[...new Set([...known.mediaIds,input.mediaId])]}
          return {id:started.jobId??`cached-${crypto.randomUUID()}`,projectDir:`biliup:${this.id}`,mediaId:input.mediaId,mediaIds:[input.mediaId],version:input.version,analyzer:started.analyzer,phase:started.state,progress:started.state==='done'?1:0,startedAt:new Date().toISOString()}
        })
        for(const job of accepted){
          const key=JSON.stringify([input.mediaId,job.analyzer])
          if(job.id.startsWith('cached-'))this.cachedAnalyses.set(key,job)
          else this.cachedAnalyses.delete(key)
        }
        this.store.set({analysisJobs:[...accepted,...jobs.filter(job=>!accepted.some(entry=>entry.id===job.id))]})
        await this.readAnalysisJobs().catch(()=>{})
      }
      return result
    }),
    'transcription.state':()=>this.safe(async()=>{
      const transcription=await this.refreshModels()
      if(!transcription)throw new Error(this.store.getSnapshot().transcriptionError??editorText('The project changed. Try the action again.'))
      return transcription
    }),
    'transcription.download':input=>this.modelAction('download',input),
    'transcription.cancelDownload':input=>this.modelAction('cancel',input),
    'transcription.selectModel':input=>this.modelAction('select',input),
    'setup.check':()=>this.safe(async()=>{const checks=await fetcher('/v1/editor/setup') as SetupCheck[];await this.refreshModels();return checks}),
    'analysis.jobs':()=>this.safe(async()=>await fetcher(`/v1/editor/projects/${this.id}/analysis`) as AnalysisJob[]),
    'media.proxies':()=>this.safe(()=>this.proxies.list()),
    'media.recent':input=>this.safe(async()=>{
      const query=new URLSearchParams();if(input.query)query.set('query',input.query);if(input.kind)query.set('kind',input.kind);if(input.limit!==undefined)query.set('limit',String(input.limit));if(input.before)query.set('before',input.before)
      return await fetcher(`/v1/editor/projects/${this.id}/recent?${query}`) as EditorCommands['media.recent']['output']
    }),
    'media.recentImport':input=>this.safe(async()=>{
      if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
      if(!this.saved||!this.history)throw new Error('工程尚未打开')
      const response=await apiFetch(`/v1/editor/projects/${this.id}/recent/${encodeURIComponent(input.id)}/import`,{method:'POST',signal:this.importAbort.signal})
      await handleResponse(response)
      const entry=await response.json() as {media:Media;source:MediaSource}
      if(this.disposed)throw new Error('工程已关闭')
      const matching=this.matchIncomingCanvas([entry.media])
      const {media,existing}=prepareImportedMedia(matching.project,entry.media,entry.source.durationMs)
      if(existing)return {mediaId:media.id,name:media.name}
      this.sources.remember(media,entry.source)
      await this.persist(editHistory(this.history,{actor:'user',ops:[...matching.ops,{op:'add_media',media}],summary:editorText('Reuse media')}),editorText('Reuse media'))
      return {mediaId:media.id,name:media.name}
    }),
    'media.proxyCancel':input=>this.safe(async()=>{await this.proxies.action(input.mediaId,'cancel')}),
    'media.proxyRetry':input=>this.safe(()=>this.proxies.action(input.mediaId,'retry')),
    'analysis.cached':input=>this.safe(async()=>{
      const query=new URLSearchParams({analyzer:input.analyzer,version:input.version})
      if(input.omit)query.set('omit',input.omit.join(','))
      if(input.model)query.set('model',input.model)
      return await fetcher(`/v1/editor/projects/${this.id}/analysis/media/${encodeURIComponent(input.mediaId)}?${query}`,{signal:this.importAbort.signal}) as {result:unknown|null}
    }),
    'analysis.cancel':input=>this.safe(async()=>{
      if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
      await handleResponse(await apiFetch(`/v1/editor/projects/${this.id}/analysis/${encodeURIComponent(input.jobId)}/cancel`,{method:'POST',signal:this.importAbort.signal}))
      this.analysisEpoch++
      await this.readAnalysisJobs().catch(()=>{})
    }),
    'export.subtitles':input=>this.safe(async()=>{
      if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
      if(this.disposed||this.history?.current.revision!==input.baseRevision)throw new Error(editorText('The project changed. Try the action again.'))
      if(this.failed||this.store.getSnapshot().biliupDirty)throw new Error(editorText('Save the current edits before exporting.'))
      const format=subtitleFormatOf(input.output)
      if(!format)throw new Error('请选择 SRT 或 WebVTT 格式')
      const project=this.history!.current.project,subtitles=subtitlesForExport(project,input.range)
      if('problem' in subtitles)throw new Error(editorText(subtitles.problem))
      const {cues,fps}=subtitles
      if(!cues.length)throw new Error(editorText(input.range?'No subtitles in this range.':'No subtitles to export.'))
      const name=input.output.split(/[\\/]/).pop()||`${project.name}.${format}`
      const url=URL.createObjectURL(new Blob([writeSubtitles(cues,fps,format)],{type:format==='vtt'?'text/vtt;charset=utf-8':'application/x-subrip;charset=utf-8'}))
      const link=document.createElement('a');link.href=url;link.download=name;link.click()
      setTimeout(()=>URL.revokeObjectURL(url),60000)
      return {path:name,format,cues:cues.length}
    }),
    'export.jobs':()=>this.safe(()=>this.readExportJobs()),
    'export.loudness':input=>this.safe(async()=>{
      if(!this.saved||this.failed||this.store.getSnapshot().biliupDirty)return {cancelled:true}
      this.cancelLoudness()
      const revision=this.saved.revision
      const active={id:input.requestId,abort:new AbortController()}
      this.loudnessRequest=active
      const current=()=>!this.disposed&&this.loudnessRequest===active&&!active.abort.signal.aborted
      this.store.set({loudnessPreview:{...this.store.getSnapshot().loudnessPreview,problem:undefined,request:{id:input.requestId,key:input.key}}})
      try{
        const value=await measureLoudness(this.id,revision,input.targetLufs,active.abort.signal,step=>{
          if(current())this.store.set({loudnessPreview:{...this.store.getSnapshot().loudnessPreview,request:{id:input.requestId,key:input.key,step}}})
        })
        if(!current()||this.saved?.revision!==revision||this.store.getSnapshot().biliupDirty||value.cancelled)return {cancelled:true}
        this.store.set({loudnessPreview:{...this.store.getSnapshot().loudnessPreview,result:{...value,key:input.key,targetLufs:input.targetLufs}}})
        return value
      }catch(error){
        if(!current())return {cancelled:true}
        this.store.set({loudnessPreview:{...this.store.getSnapshot().loudnessPreview,problem:error instanceof Error?error.message:String(error)}})
        throw error
      }finally{if(this.loudnessRequest===active)this.cancelLoudness(input.requestId)}
    }),
    'export.loudnessCancel':input=>this.safe(async()=>{this.cancelLoudness(input.requestId)}),
    'export.start':input=>this.safe(async()=>{
      if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
      if(this.disposed||this.history?.current.revision!==input.baseRevision)throw new Error(editorText('The project changed. Try the action again.'))
      if(this.failed||this.store.getSnapshot().biliupDirty)throw new Error(editorText('Save the current edits before exporting.'))
      const {baseRevision: _revision, ...options}=input
      const response=await apiFetch(`/v1/editor/projects/${this.id}/exports`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({revision:this.saved!.revision,options})})
      return this.acceptExport(response)
    }),
    'export.cancel':input=>this.safe(async()=>{
      if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
      const id=this.exportId(input.jobId)
      await handleResponse(await apiFetch(`/v1/editor/projects/${this.id}/exports/${id}/cancel`,{method:'POST'}))
      this.exportEpoch++
      await this.readExportJobs().catch(()=>{})
    }),
    'export.retry':input=>this.safe(async()=>{
      if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
      const id=this.exportId(input.jobId)
      return this.acceptExport(await apiFetch(`/v1/editor/projects/${this.id}/exports/${id}/retry`,{method:'POST'}))
    }),
    'project.get': async () => ({ ok: true, value: this.snapshot() }),
    'agent.subscribe':()=>this.safe(()=>this.refreshAgent()),
    'agent.send':input=>this.safe(()=>this.sendAgent(input)),
    'agent.undoTurn':input=>this.safe(async()=>{
      if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
      if(!this.saved||this.failed||this.store.getSnapshot().biliupDirty)throw new Error('请先处理未保存的编辑，再撤销助手操作')
      if(this.store.getSnapshot().agent?.running)throw new Error('请先停止或等待本轮完成')
      this.agentEpoch++
      this.store.set({biliupSaving:true})
      try{
        const remote=await this.agentRequest<SavedProject>(`/turns/${encodeURIComponent(input.turnId)}/undo`)
        await this.applyRemoteProject(remote)
        const current=this.store.getSnapshot().agent
        if(!this.disposed&&current)this.store.set({agent:{...current,turns:current.turns.map(turn=>turn.id===input.turnId?{...turn,undoRevision:null,undo:{state:'undone'},undoneAt:new Date(remote.updated_at).toISOString()}:turn)}})
        this.waitingAgentUi=undefined
        if(!this.disposed)this.store.set({agentChange:undefined})
        return this.snapshot()
      }finally{this.agentEpoch++;if(!this.disposed){this.store.set({biliupSaving:false});void this.refreshAgent().catch(()=>{})}}
    }),
    'agent.planSave':input=>this.safe(async()=>{
      if(!this.saved||this.failed)throw new Error('请先保存当前工程')
      const displayed=this.store.getSnapshot().agent?.plan
      if(displayed?.revision!==input.revision)throw new Error(editorText('The plan changed. Reopen it before saving your changes.'))
      this.agentEpoch++
      try {
        const plan=await this.agentRequest<AgentPlan>('/plan',{...input,projectRevision:this.saved.revision},'PUT')
        const agent=this.store.getSnapshot().agent
        if(!this.disposed&&agent&&(agent.plan?.revision??-1)<=plan.revision)this.store.set({agent:{...agent,plan},agentError:undefined})
        return plan
      } finally {
        this.agentEpoch++
        void this.refreshAgent().catch(()=>{})
      }
    }),
    'agent.approve':input=>this.safe(async()=>{
      this.agentEpoch++
      try {
        return await this.agentRequest<{accepted:boolean}>('/approve',input)
      } finally {
        this.agentEpoch++
        await this.refreshAgent().catch(()=>{})
      }
    }),
    'agent.cancel':()=>this.safe(async()=>{
      const queued=this.store.getSnapshot().agentQueued
      if(queued)this.store.set({agentQueued:{...queued,paused:true}})
      this.agentEpoch++
      try{await this.agentRequest('/cancel')}finally{this.agentEpoch++;void this.refreshAgent().catch(()=>{})}
    }),
    'agent.screenTake':input=>this.safe(async()=>{await this.agentRequest('/screen-take',input)}),
    'agent.more':()=>this.safe(async()=>{
      const cursor=this.store.getSnapshot().agent?.historyCursor
      if(!cursor)return
      const page=await readAgentPage(this.id,this.importAbort.signal,cursor)
      if(this.disposed)return
      const current=this.store.getSnapshot().agent
      if(!current||current.historyCursor?.id!==cursor.id||current.historyCursor?.createdAt!==cursor.createdAt)return
      this.store.set({agent:{...current,turns:mergeAgentTurns(page.turns,current.turns,current.projectRevision,current.running),historyCursor:page.historyCursor}})
    }),
    'agent.newThread':()=>this.safe(async()=>{
      const queued=this.store.getSnapshot().agentQueued
      if(queued)this.store.set({agentQueued:{...queued,paused:true}})
      const result=await this.agentRequest<{threadStart:string}>('/thread')
      await this.refreshAgent().catch(()=>{})
      return result
    }),
    'project.close': async () => { this.close(); return { ok: true, value: undefined } },
    'project.edit': input => this.safe(async () => {
      if (!this.history) throw new Error('工程尚未打开')
      const problems = validateOps(input.ops)
      if (problems.length) throw new Error(problems.map(problem => problem.message).join('；'))
      // validateOps verifies the unknown wire values before the typed engine sees them.
      const edit: Edit = { actor: 'user', ops: input.ops as Edit['ops'], baseRevision: input.baseRevision, summary: input.summary }
      return this.persist(editHistory(this.history, edit), input.summary ?? '编辑时间轴')
    }),
    'project.undo': () => this.safe(()=>this.stepHistory('undo')),
    'project.redo': () => this.safe(()=>this.stepHistory('redo')),
    'history.more':()=>this.safe(async()=>{await this.revisions.more();const history=this.revisions.state(this.saved!.revision);if(!this.disposed)this.store.set({history});return history}),
    'history.preview': input => this.safe(async () => ({ revision: input.revision, document: await loadRevision(this.id, input.revision) })),
    'history.jump':input=>this.safe(()=>this.stepHistory('jump',input.revision,input.baseRevision)),
    'recents.list':()=>this.safe(async()=>{const projects=await fetcher('/v1/editor/projects') as ProjectSummary[];return projects.sort((a,b)=>b.updated_at-a.updated_at).slice(0,20).map(project=>({dir:`biliup:${project.id}`,name:project.name,openedAt:new Date(project.updated_at).toISOString()}))}),
    'brand.get':()=>this.safe(()=>this.resources.brand()),
    'brand.saveKit':input=>this.safe(()=>this.resources.saveKit(input.kit)),
    'brand.deleteKit':input=>this.safe(()=>this.resources.deleteKit(input.id,input.scope)),
    'brand.saveSkill':input=>this.safe(()=>this.resources.saveSkill(input.skill)),
    'brand.deleteSkill':input=>this.safe(()=>this.resources.deleteSkill(input.id,input.scope)),
    'brand.setActive':input=>this.safe(()=>this.resources.active(input)),
    'brand.addFile':input=>this.safe(()=>this.resources.addFile(input.kit,input.scope,input.path)),
    'brand.import':input=>this.safe(()=>this.resources.importFiles(input.kit,input.scope,input.source)),
    'templates.list':()=>this.safe(()=>this.resources.templates()),
    'templates.save':input=>this.safe(()=>this.resources.saveTemplate(input.template)),
    'templates.delete':input=>this.safe(()=>this.resources.deleteTemplate(input.id)),
    'templates.fill':input=>this.safe(()=>this.resources.fill(input.id)),
  }
  private exportId(jobId:string){
    const match=/^editor-export-([1-9]\d*)$/.exec(jobId)
    if(!match)throw new Error('导出任务无效')
    return match[1]
  }
  private async acceptExport(response:Response){
    await handleResponse(response)
    const result=await response.json() as {jobId:string}
    this.exportEpoch++
    if(!this.disposed)this.store.set({exportJobId:result.jobId})
    await this.readExportJobs().catch(()=>{})
    return result
  }
  private enqueue<T>(work: () => Promise<T>,trackPending=true): Promise<T> {
    // A read barrier may wait for saves without making an idle project look unsaved.
    if(trackPending)this.pending++
    const result = this.queue.then(work).finally(() => { if(trackPending)this.pending-- })
    this.queue = result.then(() => undefined, () => undefined)
    return result
  }
  private async writeGraphic(input:EditorCommands['graphics.write']['input']):Promise<EditorCommands['graphics.write']['output']>{
    if(this.disposed||!this.canEdit())throw new Error('当前工程不可编辑')
    const response=await apiFetch(`/v1/editor/projects/${this.id}/graphics`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(input),signal:this.importAbort.signal});await handleResponse(response);return response.json()
  }
  private modelAction(action:'download'|'cancel'|'select',input:{readonly model?:string}){return this.safe(async()=>{
    if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
    this.modelsEpoch++;clearTimeout(this.modelsPoll)
    try{await handleResponse(await apiFetch(`/v1/editor/transcription/${action}`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(input),signal:this.importAbort.signal}))}
    finally{await this.refreshModels()}
  })}
  run = <K extends keyof EditorCommands>(name: K, input: EditorCommands[K]['input']): Promise<Result<EditorCommands[K]['output']>> => {
    const invoke=():Promise<Result<EditorCommands[K]['output']>>=>{
      if(this.disposed)return Promise.resolve({ok:false,error:{_tag:'NotConnected',message:'工程已关闭'}})
      return this.handlers[name](input)
    }
    // Check again when queued work starts; changing projects invalidates work
    // which has not begun, while an already submitted save may finish normally.
    return this.disposed||independentCommands.has(name)?invoke():this.enqueue(invoke)
  }
}
