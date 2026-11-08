// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Commands implemented by biliup's current editor session.
import type { Brief, StoryboardBeat, Storyboard, IntendedFinding, AgentPlan, AgentAnswer, AgentEffort, AgentAccess, AgentImage, AgentMention, AgentState, Analyzer, AnalysisJob, TranscriptionState, BrandScope, BrandKit, BrandSkill, BrandActive, BrandState, BrandImported, TakeKind, MultiLayout, MultiPart, ExportQuality, ExportLoudness, ExportRange, ExportVideo, ExportJob, ExportSubtitles, KeyProvider, KeyInfo, GeneratorInfo, GenerateRequest, GenerateJob, LibrarySearchResult, LibraryState, SoundFolderSchema, Empty, MediaRecentPage, ProxyState, SetupCheck, ProjectSnapshot, HistoryState, Problem, RecentProject, TemplateInfo, TemplateFilled } from './contracts'
import type { Result } from './state/connection'
import type { EditorDocument } from './project'

export interface EditorCommands {
  "agent.screenTake": { input: { readonly id: string; readonly path?: string; readonly source?: string; readonly problem?: string; }; output: void };
  "export.start": { input: { readonly baseRevision: number; readonly output: string; readonly quality: ExportQuality; readonly loudness: ExportLoudness; readonly targetLufs?: number; readonly subtitles?: ExportSubtitles; readonly burnCaptions?: boolean; readonly range?: ExportRange; readonly video?: ExportVideo; readonly motionBlur?: boolean; }; output: { readonly jobId: string; } };
  "export.jobs": { input: Empty; output: ReadonlyArray<ExportJob> };
  "export.cancel": { input: { readonly jobId: string; }; output: void };
  "export.retry": { input: { readonly jobId: string; }; output: { readonly jobId: string; } };
  "export.subtitles": { input: { readonly baseRevision: number; readonly output: string; readonly range?: ExportRange; }; output: { readonly path: string; readonly format: "srt" | "vtt"; readonly cues: number; } };
  "export.loudness": { input: { readonly requestId:string; readonly key:string; readonly targetLufs: number; }; output: import('./export/loudness').LoudnessResult };
  "export.loudnessCancel": {input:{readonly requestId:string};output:void};
  "analysis.start": { input: { readonly restoreJob?: string; readonly mediaId: string; readonly version: string; readonly model?: string; readonly analyzers?: ReadonlyArray<string>; }; output: ReadonlyArray<{ readonly analyzer: Analyzer; readonly state: "done" | "queued" | "running"; readonly jobId?: string; }> };
  "analysis.jobs": { input: Empty; output: ReadonlyArray<AnalysisJob> };
  "analysis.cancel": { input: { readonly jobId: string; }; output: void };
  "analysis.cached": { input: { readonly version: string; readonly model?: string; readonly mediaId: string; readonly analyzer: Analyzer; readonly omit?: ReadonlyArray<string>; }; output: { readonly result: (unknown) | null; } };
  "history.more": { input: Empty; output: HistoryState };
  "history.preview": { input: { readonly revision: number; }; output: { readonly revision: number; readonly document: EditorDocument; } };
  "history.jump": { input: { readonly revision: number; readonly baseRevision: number; }; output: ProjectSnapshot };
  "transcription.state": { input: Empty; output: TranscriptionState };
  "transcription.download": { input: { readonly model?: string; }; output: void };
  "transcription.cancelDownload": { input: { readonly model?: string; }; output: void };
  "transcription.selectModel": { input: { readonly model: string; }; output: void };
  "capture.save": { input: { readonly path: string; readonly kind: TakeKind; readonly name?: string; readonly device?: string; readonly placeAt?: number; readonly recoveryId:string;readonly placeFps:number }; output: { readonly mediaId: string; readonly name: string; readonly seconds: number; readonly itemId?: string; readonly trackId?: string; } };
  "capture.saveMulti": { input: { readonly parts: ReadonlyArray<MultiPart>; readonly name?: string; readonly placeAt?: number; readonly layout?: MultiLayout; readonly recoveryId:string;readonly placeFps:number }; output: { readonly name: string; readonly seconds: number; readonly media: ReadonlyArray<{ readonly mediaId: string; readonly kind: TakeKind; readonly name: string; readonly file: string; }>; readonly items: ReadonlyArray<{ readonly itemId: string; readonly trackId: string; readonly kind: TakeKind; readonly start: number; readonly in: number; readonly duration: number; }>; readonly group?: string; } };
  "generate.list": { input: Empty; output: ReadonlyArray<GeneratorInfo> };
  "generate.start": { input: GenerateRequest; output: { readonly jobId: string; } };
  "generate.jobs": { input: Empty; output: ReadonlyArray<GenerateJob> };
  "generate.cancel": { input: { readonly jobId: string; }; output: void };
  "generate.import": { input: { readonly jobId: string; }; output: void };
  "graphics.write": {input:{readonly name?:string;readonly html?:string;readonly from?:string;readonly edits?:ReadonlyArray<{old:string;new:string}>};output:{src:string;name:string}};
  "graphics.read": {input:{readonly src:string};output:{src:string;name:string;html:string}};
  "keys.list": { input: Empty; output: ReadonlyArray<KeyInfo> };
  "keys.set": { input: { readonly provider: KeyProvider; readonly key: string; }; output: ReadonlyArray<KeyInfo> };
  "keys.remove": { input: { readonly provider: KeyProvider; }; output: ReadonlyArray<KeyInfo> };
  "project.get": { input: Empty; output: ProjectSnapshot };
  "project.close": { input: Empty; output: void };
  "project.edit": { input: { readonly ops: ReadonlyArray<unknown>; readonly baseRevision?: number; readonly summary?: string; }; output: ProjectSnapshot };
  "project.undo": { input: Empty; output: ProjectSnapshot };
  "project.redo": { input: Empty; output: ProjectSnapshot };
  "recents.list": { input: Empty; output: ReadonlyArray<RecentProject> };
  "library.search": { input: { readonly kind: "music" | "sfx"; readonly query: string; readonly source?: "openverse" | "freesound"; readonly license?: "cc0" | "by"; readonly length?: "short" | "medium" | "long"; readonly songs?: boolean; readonly page?: number; }; output: LibrarySearchResult };
  "library.add": { input: { readonly id: string; readonly frame:number; readonly fps:number; readonly music:boolean; }; output: {readonly itemId:string;readonly frame:number}; };
  "library.state": { input: Empty; output: LibraryState };
  "library.folders": { input: Empty; output: ReadonlyArray<SoundFolderSchema> };
  "library.folderAdd": { input: { readonly selection: string; readonly target?: {readonly id:string;readonly version:number}; }; output: ReadonlyArray<SoundFolderSchema> };
  "library.folderRemove": { input: { readonly id: string; }; output: ReadonlyArray<SoundFolderSchema> };
  "media.proxies": { input: Empty; output: ReadonlyArray<ProxyState> };
  "media.proxyCancel": { input: {readonly mediaId:string}; output: void };
  "media.proxyRetry": { input: {readonly mediaId:string}; output: ReadonlyArray<ProxyState> };
  "setup.check": { input: Empty; output: ReadonlyArray<SetupCheck> };
  "agent.subscribe": { input: Empty; output: (AgentState) | null };
  "agent.more": { input: Empty; output: void };
  "agent.send": { input: { readonly prompt: string; readonly provider?: string; readonly frame?: number; readonly selected?: string; readonly mentions?: ReadonlyArray<AgentMention>; readonly model?: string; readonly effort?: AgentEffort; readonly access?: AgentAccess; readonly images?: ReadonlyArray<AgentImage>; readonly skipStoryboard?: boolean; readonly ranges?: { readonly marks?: ExportRange; readonly selection?: ExportRange; }; }; output: { readonly turnId: string; } };
  "agent.cancel": { input: Empty; output: void };
  "agent.undoTurn": { input: { readonly turnId: string; }; output: ProjectSnapshot };
  "media.recent": { input: { readonly query?: string; readonly kind?: string; readonly limit?: number; readonly before?: string; }; output: MediaRecentPage };
  "media.recentImport": { input: { readonly id: string; }; output: { readonly mediaId: string; readonly name: string; } };
  "agent.newThread": { input: Empty; output: { readonly threadStart: string; } };
  "agent.approve": { input: { readonly id: string; readonly allow: boolean; readonly answer?: AgentAnswer; readonly beats?: ReadonlyArray<StoryboardBeat>; readonly note?: string; }; output: { readonly accepted: boolean; } };
  "agent.planSave": { input: { readonly revision: number; readonly brief?: Brief; readonly storyboard?: Storyboard; readonly intended?: ReadonlyArray<IntendedFinding>; }; output: AgentPlan };
  "brand.get": { input: Empty; output: BrandState };
  "brand.saveKit": { input: { readonly kit: BrandKit; }; output: BrandState };
  "brand.deleteKit": { input: { readonly id: string; readonly scope: BrandScope; }; output: BrandState };
  "brand.saveSkill": { input: { readonly skill: BrandSkill; }; output: BrandState };
  "brand.deleteSkill": { input: { readonly id: string; readonly scope: BrandScope; }; output: BrandState };
  "brand.setActive": { input: BrandActive; output: BrandState };
  "brand.import": { input: { readonly kit: string; readonly scope: BrandScope; readonly source: string; }; output: { readonly files: ReadonlyArray<BrandImported>; readonly failed?: ReadonlyArray<{name:string;error:string}>; readonly skipped?: number; readonly state: BrandState; } };
  "brand.addFile": { input: { readonly kit: string; readonly scope: BrandScope; readonly path: string; }; output: { readonly name: string; readonly state: BrandState; } };
  "templates.list": { input: {  }; output: ReadonlyArray<TemplateInfo> };
  "templates.save": { input: { readonly template: TemplateInfo; }; output: ReadonlyArray<TemplateInfo> };
  "templates.delete": { input: { readonly id: string; }; output: ReadonlyArray<TemplateInfo> };
  "templates.fill": { input: { readonly id: string; }; output: TemplateFilled };
}

export type Handlers = { [K in keyof EditorCommands]: (input: EditorCommands[K]['input']) => Promise<Result<EditorCommands[K]['output']>> }
