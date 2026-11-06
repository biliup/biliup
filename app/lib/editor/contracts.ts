// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Plain editor DTOs and catalog constants. Services are owned by biliup; no Effect/RPC runtime.
import type { CaptureKind } from './core/media/capture';
export const REVIEW_SCORES = ["hook", "readability", "motion", "variety", "sound", "brand"] as const;
export const AGENT_EFFORTS = ["low", "medium", "high", "max"] as const;
export const AGENT_ACCESS = ["look", "ask", "auto"] as const;
export const AGENT_IMAGE_TYPES = ["image/png", "image/jpeg", "image/gif", "image/webp"] as const;
export const AGENT_IMAGE_MAX = 5 * 1024 * 1024;
export const AGENT_IMAGES_MAX = 4;
export const LIBRARY_PANELS = ["media", "transcript", "text", "elements", "audio", "sfx", "transitions", "effects", "capture", "generate", "brand", "skills"] as const;
export type LibraryPanel = (typeof LIBRARY_PANELS)[number];
export const isLibraryPanel = (panel: string): panel is LibraryPanel => (LIBRARY_PANELS as ReadonlyArray<string>).includes(panel);
export const AGENT_UI_PANELS = [...LIBRARY_PANELS, "inspector", "agent", "history"] as const;
export const ANALYZERS = ["loudness", "video", "transcript", "beats"] as const;
export const takeUploadName = (kind: TakeKind, ext: "webm" | "mp4") => `biliup-take-${kind}-${Array.from(crypto.getRandomValues(new Uint8Array(6)), (b) => b.toString(16).padStart(2, "0")).join("")}.${ext}`;
export const TAKE_UPLOAD_NAME = /^biliup-take-(voiceover|camera|screen|system)-[0-9a-f]{12}(?: \d+)?\.(?:webm|mp4)$/;
export const DEFAULT_LINK_QUALITY = "1080";
export const FILMSTRIP_TILE_H = 64;
export const AGENT_IMAGE_NAME = /^turn-[a-z0-9]+-[a-f0-9]{6}-(\d|s\d{1,2}-\d)\.(png|jpg|gif|webp)$/;
export type Brief = { readonly film: string; readonly audience: string; readonly duration: string; readonly formats: ReadonlyArray<string>; readonly assets: ReadonlyArray<string>; readonly visual: string; readonly hardLines: ReadonlyArray<string>; readonly deliverables: ReadonlyArray<string>; readonly notes?: string; readonly updatedAt?: string; readonly by?: "agent" | "user"; readonly type?: string; readonly skills?: ReadonlyArray<string>; };
export type StoryboardBeat = { readonly start: number; readonly end: number; readonly title: string; readonly entry: string; readonly exit: string; readonly purpose: string; readonly assets: ReadonlyArray<string>; readonly sound?: string; readonly still?: string; };
export type Storyboard = { readonly summary: string; readonly beats: ReadonlyArray<StoryboardBeat>; readonly status: "proposed" | "approved" | "declined"; readonly approvedBy?: "user" | "setting" | "agent"; readonly note?: string; readonly updatedAt?: string; };
export type ReviewScores = { readonly hook?: number; readonly readability?: number; readonly motion?: number; readonly variety?: number; readonly sound?: number; readonly brand?: number; };
export type Review = { readonly round: number; readonly scores: ReviewScores; readonly defects: ReadonlyArray<{ readonly at: number; readonly what: string; readonly fix?: string; readonly id?: string; readonly kind?: string; readonly source?: string; readonly rule?: string; readonly to?: number; }>; readonly notes?: string; readonly by?: "critic" | "agent"; readonly blockers?: ReadonlyArray<{ readonly id?: string; readonly kind: string; readonly what: string; readonly at?: number; readonly source: string; readonly rule?: string; readonly to?: number; readonly fix?: string; }>; readonly rubric?: Readonly<Record<string, "required" | "optional" | "na">>; readonly videoType?: string; readonly soundProfile?: string; readonly intended?: number; readonly evidence?: Readonly<Record<string, string>>; readonly fixes?: ReadonlyArray<{ readonly defect: string; readonly what: string; readonly at?: number; readonly verified: boolean; readonly note?: string; }>; readonly error?: string; readonly determinism?: { readonly checked: ReadonlyArray<string>; readonly differed: ReadonlyArray<string>; }; readonly looked?: { readonly sheet: boolean; readonly phone: boolean; }; readonly passed: boolean; readonly revision?: number; readonly at: string; readonly turnId?: string; };
export type IntendedFinding = { readonly kind: string; readonly what: string; readonly from?: number; readonly to?: number; readonly at: string; };
export type AgentPlan = { readonly revision: number; readonly brief?: Brief; readonly storyboard?: Storyboard; readonly reviews: ReadonlyArray<Review>; readonly intended?: ReadonlyArray<IntendedFinding>; };
export type AgentQuestion = { readonly question: string; readonly options: ReadonlyArray<{ readonly label: string; readonly detail?: string; }>; readonly multi: boolean; readonly freeText: boolean; };
export type AgentAnswer = { readonly choices: ReadonlyArray<string>; readonly text?: string; };
export type AgentCard = { readonly type: "question" | "brief" | "storyboard" | "review" | "brand"; readonly question?: AgentQuestion; readonly answer?: AgentAnswer; readonly outcome?: string; readonly brief?: Brief; readonly storyboard?: Storyboard; readonly review?: Review; readonly text?: string; };
export type AgentItem = { readonly kind: "text" | "step" | "thinking" | "card"; readonly text: string; readonly tool?: string; readonly status?: "running" | "done" | "error"; readonly live?: boolean; readonly error?: string; readonly doing?: string; readonly result?: string; readonly card?: AgentCard; };
export type AgentEffort = (typeof AGENT_EFFORTS)[number];
export type AgentAccess = (typeof AGENT_ACCESS)[number];
export type AgentImage = { readonly name: string; readonly mimeType: (typeof AGENT_IMAGE_TYPES)[number]; readonly data: string; };
export type AgentEdit = { readonly revision: number; readonly summary: string; readonly lines: ReadonlyArray<string>; readonly at?: number; readonly media?: ReadonlyArray<string>; };
export type AgentTurnStatus = "running" | "done" | "error" | "cancelled";
export type AgentMentionKind = "media" | "item" | "track" | "marker" | "moment" | "range" | "effect" | "transition" | "region";
export type AgentMention = { readonly kind: AgentMentionKind; readonly id: string; readonly label: string; readonly range?: { readonly start: number; readonly end: number; }; readonly frame?: number; readonly area?: { readonly x: number; readonly y: number; readonly w: number; readonly h: number; }; };
export type AgentUndo = { readonly state: "none" | "available" | "blocked" | "unavailable" | "undone"; readonly reason?: string; };
export type AgentFolder = { readonly path: string; readonly files: number; readonly folders: number; readonly problem?: string; };
export type AgentFile = { readonly id:string; readonly projectId:number; readonly revision:number; readonly name:string; readonly format:"srt"|"vtt"|"mp4"; readonly cues:number; readonly exportJob?:number; };
export type AgentTurn = { readonly undoRevision?:number|null; readonly files?:ReadonlyArray<AgentFile>; readonly id: string; readonly prompt: string; readonly mentions?: ReadonlyArray<{ readonly kind: AgentMentionKind; readonly label: string; }>; readonly note?: string; readonly images?: ReadonlyArray<string>; readonly model?: string; readonly folders?: ReadonlyArray<AgentFolder>; readonly startedAt: string; readonly finishedAt?: string; readonly status: AgentTurnStatus; readonly items: ReadonlyArray<AgentItem>; readonly edits: ReadonlyArray<AgentEdit>; readonly error?: string; readonly seconds?: number; readonly undoneAt?: string; readonly undo: AgentUndo; readonly spend?: number; readonly graphics?: ReadonlyArray<string>; readonly cleanup?: { readonly tracks: ReadonlyArray<string>; }; };
export type AgentUiPanel = (typeof AGENT_UI_PANELS)[number];
export type AgentUi = { readonly id: string; readonly revision: number; readonly frame?: number; readonly select?: string; readonly panel?: AgentUiPanel; };
export type AgentApproval = { readonly id: string; readonly turnId: string; readonly text: string; readonly estimate?: number; readonly until: string; readonly screen?: { readonly source?: string; readonly seconds: number; readonly withMic: boolean; }; readonly edit?: { readonly lines: ReadonlyArray<string>; }; readonly question?: AgentQuestion; readonly storyboard?: Storyboard; readonly brand?: { readonly kit: string; readonly changes: ReadonlyArray<{ readonly field: string; readonly before: ReadonlyArray<string>; readonly after: ReadonlyArray<string>; }>; }; };
export type AgentContext = { readonly used: number; readonly window?: number; readonly systemTools: number; readonly conversation: number; readonly message: number; readonly model?: string; };
export type AgentHistoryCursor = {readonly createdAt:number;readonly id:string};
export type AgentState = { readonly historyBranch?:number; readonly historyCursor?:AgentHistoryCursor|null; readonly resourceVersion?:number; readonly shaderVersion?:number; readonly dir: string; readonly provider: string; readonly model?: string; readonly configured?: boolean; readonly projectRevision?: number; readonly running: (string) | null; readonly turns: ReadonlyArray<AgentTurn>; readonly ui?: AgentUi; readonly approval?: AgentApproval; readonly screenTake?: string; readonly plan?: AgentPlan; readonly warning?: string; readonly context?: AgentContext; readonly threadStart?: string; };
export type AgentError = { readonly _tag: "AgentError"; readonly reason: "no-project" | "busy" | "not-installed" | "unknown-turn" | "failed"; readonly message: string; };
export type Analyzer = (typeof ANALYZERS)[number];
export type AnalysisJob = { readonly version: string; readonly id: string; readonly projectDir: string; readonly mediaId: string; readonly mediaIds: ReadonlyArray<string>; readonly analyzer: Analyzer | "restore"; readonly phase: "queued" | "running" | "done" | "failed" | "cancelled"; readonly progress: number; readonly waiting?: "export"; readonly stage?: "model"; readonly error?: string; readonly startedAt: string; readonly finishedAt?: string; };
export type AnalysisError = { readonly _tag: "AnalysisError"; readonly reason: "no-project" | "unknown-media" | "unknown-analyzer" | "failed"; readonly message: string; };
export type WhisperModel = { readonly id: string; readonly name: string; readonly bytes: number; readonly multilingual: boolean; readonly offered: boolean; readonly installed: boolean; };
export type ModelDownload = { readonly model: string; readonly phase: "downloading" | "done" | "failed" | "cancelled"; readonly received: number; readonly total?: number; readonly error?: string; };
export type TranscriptionState = { readonly selected: string; readonly dir: string; readonly models: ReadonlyArray<WhisperModel>; readonly downloads: ReadonlyArray<ModelDownload>; };
export type TranscriptionError = { readonly _tag: "TranscriptionError"; readonly reason: "unknown-model" | "failed"; readonly message: string; };
export type BrandColor = { readonly hex: string; readonly role: string; readonly name?: string; };
export type BrandFont = { readonly family: string; readonly role: string; readonly weight?: number; readonly file?: string; };
export type BrandStyle = { readonly source?: string; readonly palette: ReadonlyArray<{ readonly hex: string; readonly role?: string; }>; readonly type: string; readonly shotLengths: string; readonly transitions: string; readonly camera: string; readonly grain: string; readonly notes?: string; };
export type BrandScope = "global" | "project";
export type BrandKit = { readonly id: string; readonly name: string; readonly scope: BrandScope; readonly logos: { readonly light?: string; readonly dark?: string; readonly mark?: string; }; readonly colors: ReadonlyArray<BrandColor>; readonly fonts: ReadonlyArray<BrandFont>; readonly about?: string; readonly tone: string; readonly taglines: ReadonlyArray<string>; readonly urls: ReadonlyArray<string>; readonly dos: ReadonlyArray<string>; readonly donts: ReadonlyArray<string>; readonly examples: ReadonlyArray<string>; readonly style?: BrandStyle; readonly updatedAt?: string; };
export type BrandSkill = { readonly id: string; readonly name: string; readonly scope: BrandScope; readonly description: string; readonly body: string; readonly updatedAt?: string; readonly builtin?: boolean; };
export type BrandActive = { readonly kit: (string) | null; readonly skills: ReadonlyArray<string>; };
export type BrandState = { readonly kits: ReadonlyArray<BrandKit>; readonly skills: ReadonlyArray<BrandSkill>; readonly active: BrandActive; readonly files: Readonly<Record<string, Readonly<Record<string, string>>>>; };
export type BrandError = { readonly _tag: "BrandError"; readonly reason: "no-project" | "invalid" | "not-found" | "failed"; readonly message: string; };
export type BrandImported = { readonly name: string; readonly kind: "logo" | "font"; readonly slot?: "light" | "dark" | "mark"; readonly family?: string; };
export type TakeKind = CaptureKind;
export type MultiLayout = "tl" | "tr" | "bl" | "br" | "side";
export type MultiPart = { readonly path: string; readonly kind: TakeKind; readonly device?: string; readonly offsetMs: number; readonly recordedMs?: number; };
export type ExportQuality = "draft" | "standard" | "high";
export type ExportLoudness = "off" | "normal" | "quiet" | "broadcast" | "target";
export type ExportRange = { readonly from: number; readonly to: number; };
export type ExportCodec = "h264" | "hevc";
export type ExportEncoder = "hardware" | "software";
export type ExportVideo = { readonly codec?: ExportCodec; readonly encoder?: ExportEncoder; readonly bitrateMbps?: number; readonly shortSide?: number; readonly fps?: number; };
export type ExportJob = {
  readonly id: string;
  readonly projectDir: string;
  readonly name: string;
  readonly output: string;
  readonly quality: ExportQuality;
  readonly codec: ExportCodec;
  readonly encoder?: ExportEncoder;
  readonly encoderName?: string;
  readonly bitrateMbps?: number;
  readonly loudness: ExportLoudness;
  readonly targetLufs?: number;
  readonly phase: "queued" | "preparing" | "rendering" | "done" | "failed" | "cancelled";
  readonly progress: number;
  readonly step?: { readonly kind: "sound" | "mix" | "graphics" | "shaders" | "loudness" | "encoder"; readonly done?: number; readonly total?: number; readonly pass?: number; };
  readonly etaSeconds?: number;
  readonly width: number;
  readonly height: number;
  readonly fps: number;
  readonly seconds: number;
  readonly startedAt: string;
  readonly finishedAt?: string;
  readonly elapsedSeconds?: number;
  readonly error?: string;
  readonly warnings?: ReadonlyArray<string>;
  readonly subtitlesFile?: string;
  readonly subtitleTrack?: string;
  readonly burnCaptions?: boolean;
  readonly range?: { readonly from: number; readonly to: number; };
  readonly motionBlur?: boolean;
  readonly agent?: boolean;
};
export type ExportSubtitles = { readonly file?: "srt" | "vtt"; readonly track?: boolean; readonly language?: string; };
export type ExportError = { readonly _tag: "ExportError"; readonly reason: "no-project" | "no-ffmpeg" | "exists" | "invalid-path" | "empty" | "missing-media" | "source-media" | "busy" | "unknown-job" | "invalid-settings" | "no-codec"; readonly message: string; };
export type GenerateKind = "image" | "video" | "audio";
export type KeyProvider = "openai" | "google" | "elevenlabs" | "freesound" | "brave";
export type KeyInfo = { readonly provider: KeyProvider; readonly name: string; readonly set: boolean; readonly storage: "keychain" | "memory" | "server"; readonly usedBy: ReadonlyArray<string>; readonly note?: string; };
export type GenerationPricing = {readonly asOf:string;readonly url:string} & (
  | {readonly kind:'image';readonly quality:string;readonly usdBySize:Readonly<Record<string,number>>}
  | {readonly kind:'video';readonly usdPerSecond:number;readonly secondsMin:number;readonly secondsMax:number}
  | {readonly kind:'audio';readonly usdPerMinute:number}
  | {readonly kind:'usage'}
);
export type GeneratorInfo = { readonly id: string; readonly name: string; readonly kinds: ReadonlyArray<GenerateKind>; readonly provider?: KeyProvider; readonly model?: string; readonly ready: boolean; readonly needs?: string; readonly needsKey?:boolean; readonly sizes: ReadonlyArray<string>; readonly seconds?: ReadonlyArray<number>; readonly pricing: GenerationPricing; };
export type GenerateRequest = { readonly generator: string; readonly prompt: string; readonly kind?: GenerateKind; readonly size?: string; readonly seconds?: number; readonly name?: string; readonly content?:string; };
export type GenerateJob = { readonly agentTurnId?:string; readonly id: string; readonly projectDir: string; readonly generator: string; readonly kind: GenerateKind; readonly prompt: string; readonly size?: string; readonly seconds?: number; readonly phase: "running" | "importing" | "done" | "failed" | "cancelled"; readonly status?: string; readonly startedAt: string; readonly estimate?: number; readonly error?: string; readonly mediaId?: string; readonly name?: string; };
export type GenerateError = { readonly _tag: "GenerateError"; readonly reason: "no-project" | "unknown-generator" | "needs-key" | "bad-request" | "keys-unavailable" | "failed"; readonly message: string; };
export type LibraryHitSchema = { readonly source: "openverse" | "freesound"; readonly id: string; readonly title: string; readonly creator: string; readonly creatorUrl?: string; readonly landingUrl: string; readonly provider?: string; readonly license: "cc0" | "by"; readonly licenseVersion: string; readonly licenseUrl: string; readonly attribution: string; readonly seconds?: number; readonly tags: ReadonlyArray<string>; readonly filetype?: string; };
export type LibraryLimit = { readonly scope: "minute" | "day"; readonly until: string; };
export type LibrarySearchResult = { readonly hits: ReadonlyArray<LibraryHitSchema>; readonly total: number; readonly page: number; readonly pages: number; readonly source: "openverse" | "freesound"; readonly cached: boolean; readonly limit?: LibraryLimit; readonly left?: { readonly minute?: number; readonly day?: number; }; readonly searched?: string; readonly alias?: string; readonly note?: string; };
export type LibraryState = { readonly sources: ReadonlyArray<"openverse" | "freesound">; readonly limit?: LibraryLimit; };
export type LibraryError = { readonly _tag: "LibraryError"; readonly reason: "bad-request" | "no-key" | "network" | "refused" | "limit" | "no-project" | "io"; readonly message: string; };
export type SoundFolderSchema = import('./core/audio/library').SoundFolder;
export type Empty = {  };
export type MediaRecentEntry = { readonly id: string; readonly path: string; readonly name: string; readonly kind: "video" | "image" | "audio"; readonly origin: "capture" | "import" | "generated"; readonly seconds?: number; readonly width?: number; readonly height?: number; readonly project: { readonly dir: string; readonly name: string; }; readonly device?: string; readonly created: string; readonly thumb?: boolean; };
export type MediaRecentPage = { readonly entries: ReadonlyArray<MediaRecentEntry>; readonly next: string | null; };
export type ProxyState = { readonly mediaId: string; readonly path?: string; readonly phase: "queued" | "running" | "done" | "failed" | "cancelled"; readonly key?: string; readonly progress?: number; readonly waiting?: "export"; readonly error?: string; };
export type MediaImportResult = { readonly imported: ReadonlyArray<{ readonly path: string; readonly id: string; readonly name: string; readonly via: string; }>; readonly subtitles?: ReadonlyArray<{ readonly path: string; readonly track: string; readonly cues: number; }>; readonly skipped: ReadonlyArray<{ readonly path: string; readonly reason: string; }>; readonly failed: ReadonlyArray<{ readonly path: string; readonly message: string; }>; };
export type ImportJob = { readonly id: string; readonly projectDir: string; readonly total: number; readonly phase: "queued" | "probing" | "adding" | "posters" | "done" | "failed"; readonly probed: number; readonly postersDone: number; readonly postersTotal: number; readonly result?: MediaImportResult; readonly error?: string; readonly agent?: boolean; };
export type SetupStatus = "ready" | "missing" | "needs-login" | "warning" | "error";
export type SetupCheck = { readonly id: string; readonly name: string; readonly status: SetupStatus; readonly optional: boolean; readonly version?: string; readonly path?: string; readonly detail: string; readonly fix?: { readonly text: string; readonly command?: string; }; };
export type MediaCoreError = { readonly _tag: "MediaCoreError"; readonly message: string; };
export type ProjectSnapshot = { readonly dir: string; readonly project: unknown; readonly revision: number; readonly canUndo: boolean; readonly canRedo: boolean; };
export type HistoryEntry = { readonly revision: number; readonly at: string; readonly kind: "initial" | "edit" | "undo-turn" | "external" | "jump"; readonly author: "user" | "agent" | "external" | "import"; readonly turnId?: string; readonly label: string; readonly lines: ReadonlyArray<string>; readonly current: boolean; };
export type HistoryState = { readonly before: number | null; readonly dir: string; readonly revision: number; readonly problem?: string; readonly total: number; readonly entries: ReadonlyArray<HistoryEntry>; };
export type Problem = { readonly code: string; readonly path: string; readonly message: string; };
export type Canvas = { readonly width: number; readonly height: number; readonly fps: 23.976 | 24 | 25 | 29.97 | 30 | 50 | 59.94 | 60; readonly background?: string; };
export type RecentProject = { readonly dir: string; readonly name: string; readonly openedAt: string; };
export type TemplateInfo = { readonly id: string; readonly name: string; readonly description: string; readonly body: string; readonly builtin?: boolean; };
export type TemplateFilled = { readonly text: string; readonly missing: ReadonlyArray<string>; };
export type TemplateError = { readonly _tag: "TemplateError"; readonly reason: "invalid" | "not-found" | "builtin" | "failed"; readonly message: string; };
export interface FilmstripLayout {
  readonly version: 1;
  /** Seconds between tiles. */
  readonly interval: number;
  /** Tiles that hold a frame (the rest of the last row is padding). */
  readonly count: number;
  readonly cols: number;
  readonly rows: number;
  readonly tileWidth: number;
  readonly tileHeight: number;
  /** The media's length (s) the strip was made for. */
  readonly seconds: number;
}
export interface Waveform {
  readonly version: 1;
  /** Peaks per second. */
  readonly rate: number;
  readonly seconds: number;
  readonly peaks: string;
}

export type SoundFolderInfo = SoundFolderSchema;
