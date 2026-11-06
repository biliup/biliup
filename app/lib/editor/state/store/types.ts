import type {EditorCommands} from "@/app/lib/editor/commands";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { ProjectSnapshot, RecentProject, SetupCheck, GeneratorInfo, GenerateJob, KeyInfo, ImportJob, MediaImportResult, AgentState, HistoryState, AgentMention, AgentImage, AgentUiPanel, ExportJob, AnalysisJob, ProxyState, TranscriptionState, BrandState, GenerateKind, TemplateInfo } from "@/app/lib/editor/contracts";
import type { ProjectModel } from "@/app/lib/editor/state/project";

export type ConnectionState = "connecting" | "connected" | "reconnecting" | "failed";
export type Project = ProjectModel;

/** A message in the editor's banner. `transport` ones go away when the connection comes back. */
export interface Notice {
  readonly text: string;
  readonly transport?: boolean;
}

export interface AppStateShape {
  readonly loudnessPreview?:import('../../export/loudness').LoudnessPreviewState;
  readonly agentUndoing?: string;
  readonly projectShaders?:ReadonlyArray<import('@/app/lib/editor/core/visual/shaders').ShaderDef>;
  readonly projectFonts?:ReadonlyArray<import('@/app/lib/editor/project-fonts').ProjectFont>;
  readonly biliupRevision?: number;
  readonly biliupOrigin?: { sessionId: number | null; inMs: number; outMs: number };
  readonly biliupSources?: Record<string,import('@/app/lib/editor/project').MediaSource>;
  readonly biliupSaving?: boolean;
  readonly biliupDirty?: boolean;
  readonly biliupSaveError?: string;
  readonly biliupSaveConflict?: boolean;
  readonly biliupCanEdit?: boolean;
  /** Canvas tool selection; saved masks live only in the project document. */
  readonly maskEditing?: { readonly itemId: string; readonly regionId?: string; readonly drawing?: boolean };
  readonly ready: boolean;
  readonly connection: ConnectionState;
  readonly connectionMessage?: string;
  readonly snapshot: ProjectSnapshot | null;
  readonly recents: ReadonlyArray<RecentProject>;
  // A readable error for the welcome screen (a failed open there).
  readonly welcomeError?: string;
  readonly notice?: Notice;
  // Settings (G8): the project's and the app's settings, and the keyboard shortcuts sheet (G3).
  readonly settingsOpen: boolean;
  readonly settingsSection: SettingsSection;
  readonly settingsFocus?: "keys" | "transcription";
  readonly shortcutsOpen: boolean;
  readonly setupChecks?: ReadonlyArray<SetupCheck>;
  readonly setupChecking: boolean;
  // The last setup.check failed as a whole (server unreachable): shown with Check again.
  readonly setupError?: string;
  /** The Generate tab (D99): the generators and whether each has its key, the server's generation jobs, a prompt to edit. */
  readonly generators?: ReadonlyArray<GeneratorInfo>;
  readonly generatorsError?: string;
  readonly generateJobs: ReadonlyArray<GenerateJob>;
  readonly generateStarting: boolean;
  readonly generateError?: string;
  readonly generateDraft?: GenerateDraft & { readonly seq: number };
  /** Settings › Keys: which providers have a key (never the key). */
  readonly keys?: ReadonlyArray<KeyInfo>;
  readonly keysError?: string;
  /** The library sound being added (D134), for its row's spinner. */
  readonly libraryAdding?: string | undefined;
  // Import jobs on the server (running and recent); the library shows the open project's.
  readonly jobs: ReadonlyArray<ImportJob>;
  readonly importReport?: { readonly failed: MediaImportResult["failed"]; readonly skipped: MediaImportResult["skipped"]; readonly warnings?: ReadonlyArray<{readonly path:string;readonly message:string}> };
  // Media ids that have a poster on the server; others show an icon (no request, no 404).
  // The open project's agent (turns, the running one) from agent.subscribe.
  readonly agent: AgentState | null;
  // The open project's history (D71) from history.subscribe, for the History tab.
  readonly history: HistoryState | null;
  // Why the last agent action failed (not installed, busy…), shown in the Agent tab.
  readonly agentError?: string;
  readonly agentSending?: boolean;
  readonly agentPlanSaving?: boolean;
  // The composer (D140): what's typed and what its @labels point at. In the store, so Add to chat
  // (D141) from the timeline, the canvas or the Inspector lands in it with the Agent tab closed.
  readonly agentDraft: string;
  readonly agentMentions: ReadonlyArray<AgentMention>;
  // A message written while a turn runs: sent when that turn ends.
  readonly agentQueued?: { readonly request: EditorCommands["agent.send"]["input"]; readonly paused?: boolean };
  // Images attached in the composer (D184), sent with the next message.
  readonly agentImages: ReadonlyArray<AgentImage>;
  // Bumped to put the caret in the composer (after Add to chat).
  readonly agentFocus: number;
  // D202, D235: an agent ui_show (it never moves the playhead): the pill's stretch, in frames.
  readonly agentChange?: { readonly from: number; readonly to: number; readonly select?: string | undefined; readonly seq: number };
  // A panel the agent's ui_show asked for (D65); the rail and the side tabs follow it once per seq.
  readonly panelRequest?: { readonly panel: AgentUiPanel; readonly seq: number };
  /** "Show in Media" (T7): the tile to scroll to and flash. */
  readonly mediaFocus?: { readonly id: string; readonly seq: number };
  // The export dialog (D34) and the server's export jobs; the one this window started last.
  readonly exportOpen: boolean;
  readonly exportSubmitting?: boolean;
  readonly exportJobs: ReadonlyArray<ExportJob>;
  readonly exportJobsLoaded?: boolean;
  readonly exportJobsError?: string;
  readonly exportJobId?: string;
  // Analysis jobs on the server (D66); the Library shows the open project's running ones per tile.
  readonly analysisJobs: ReadonlyArray<AnalysisJob>;
  readonly analysisStarting?: ReadonlyArray<string>;
  readonly analysisCancelling?: ReadonlyArray<string>;
  /** D212: the open project's preview proxies (media that needs none has no entry). */
  readonly proxies: ReadonlyArray<ProxyState>;
  // Why the last Analyze click was refused, shown in the Library.
  readonly analysisError?: string;
  // The speech models for transcripts and their downloads (D68), for the Setup row.
  readonly transcription?: TranscriptionState;
  // Why the last model action was refused.
  readonly transcriptionError?: string;
  readonly transcriptionPending?: boolean;
  /** Browser-selected files being uploaded to the current deployment. */
  readonly uploading?: { readonly total: number; readonly done: number; readonly name: string };
  /** D186: the brand kits and skills the open project sees, and its choice (the Brand tab). */
  readonly brand?: BrandState;
  /** Why the last brand action failed, shown in the Brand tab. */
  readonly brandError?: string;
  /** Issue #111: the templates the composer's / menu offers (Biliup's built-ins, then the user's own). */
  readonly templates?: ReadonlyArray<TemplateInfo>;
}


export const MAX_NAME_LENGTH = 100;

/** What the Generate tab's form holds. */
export interface GenerateDraft {
  readonly generator: string;
  readonly prompt: string;
  readonly kind?: GenerateKind | undefined;
  readonly size?: string | undefined;
  readonly seconds?: number | undefined;
}
/**
 * A field where keys type text (text-like inputs, textareas, contenteditable). A focused checkbox,
 * colour picker or slider is not one: ⌘Z there undoes the project, not typing.
 */
export type SettingsSection = "project" | "app" | "services" | "shortcuts";
