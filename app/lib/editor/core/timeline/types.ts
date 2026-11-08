// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Types for the Biliup project format, edit operations and built-in module definitions.
// Written by hand and kept pragmatic: one Item shape with every per-type field optional rather
// than a union per item type. The schemas stay the source of truth; the validator enforces them.
// Format types are readonly (they describe data); the edit engine works on a Mutable copy.

export type Id = string;
export type ModuleId = string;
/** Frames from a start, at the canvas fps. */
export type Frame = number;

export type Mutable<T> = T extends ReadonlyArray<infer U> ? Array<Mutable<U>> : T extends object ? { -readonly [K in keyof T]: Mutable<T[K]> } : T;

/** The canvas frame rates (D209): the NTSC ones are stored as 23.976, 29.97 and 59.94. */
export const CANVAS_FPS = [23.976, 24, 25, 29.97, 30, 50, 59.94, 60] as const;
export type CanvasFps = (typeof CANVAS_FPS)[number];

export interface Canvas {
  readonly width: number;
  readonly height: number;
  readonly fps: CanvasFps;
  readonly background?: string;
  /** Match the first imported video while the project is still empty. */
  readonly matchFirstClip?: boolean;
}

export interface ModuleUse {
  readonly module: ModuleId;
  readonly params?: { readonly [key: string]: unknown };
  readonly bypass?: boolean;
}

export interface TimedModule {
  readonly module: ModuleId;
  readonly duration: Frame;
  readonly params?: { readonly [key: string]: unknown };
}

export interface MediaProbe {
  readonly frames?: number;
  readonly fps?: number;
  readonly width?: number;
  readonly height?: number;
  readonly hasAudio?: boolean;
}

export interface Media {
  readonly id: Id;
  readonly kind: "video" | "image" | "audio";
  readonly name: string;
  readonly path: string;
  readonly source?: { readonly module: ModuleId; readonly [key: string]: unknown };
  readonly probe?: MediaProbe;
}

export interface Transform {
  readonly x?: number;
  readonly y?: number;
  readonly w?: number;
  readonly h?: number;
  readonly scale?: number;
  readonly rotate?: number;
  readonly opacity?: number;
  readonly fit?: "contain" | "cover" | "fill";
  /** Clips: corner radius of the box in canvas pixels (D75). */
  readonly radius?: number;
}

export interface TextStyle {
  readonly preset?: ModuleId;
  readonly font?: string;
  readonly size?: number;
  readonly weight?: number;
  readonly tracking?: number;
  readonly lineHeight?: number;
  readonly color?: string;
  readonly align?: "left" | "center" | "right";
  readonly uppercase?: boolean;
  /** D129: lower and title case (upper is `uppercase`); wins over uppercase when set. */
  readonly case?: "lower" | "title";
  /** D129: slanted (the font's italic, else a synthesized oblique). */
  readonly italic?: boolean;
  /** D80: false = none (even when the preset's look has one); absent = the look's. */
  readonly stroke?: TextStroke | false;
  readonly shadow?: TextShadow | false;
  readonly background?: TextBackground | false;
}

/** A text's outline (D80): width in canvas px, colour a theme token or CSS colour. */
export interface TextStroke {
  readonly width: number;
  readonly color?: string;
}
/** A text's drop shadow (D80): offsets and blur in canvas px, opacity 0–1. */
export interface TextShadow {
  readonly x?: number;
  readonly y?: number;
  readonly blur?: number;
  readonly color?: string;
  readonly opacity?: number;
}
/** A box behind a text (D80): one box around its text box, or one behind each line; padding (the sides; top and bottom get half) and radius in canvas px. */
export interface TextBackground {
  readonly color?: string;
  readonly opacity?: number;
  readonly padding?: number;
  readonly radius?: number;
  readonly wrap?: "box" | "lines";
}

export type ItemType = "clip" | "component" | "graphic" | "text" | "captions" | "audio" | "danmaku";

export interface Item {
  readonly id: Id;
  readonly type: ItemType;
  readonly name?: string;
  /** Free tracks only; main-track items are placed by their order and `gap`. */
  readonly start?: Frame;
  /**
   * Main track only (D224): empty frames before the item, after the one before it (or frame 0).
   * Absent = 0, back to back. A gap shows black and is silent; a transition can't cross one.
   */
  readonly gap?: Frame;
  readonly duration: Frame;
  readonly locked?: boolean;
  /** D157: items with the same group move, trim and delete together on the timeline (a kicker and its title). */
  readonly group?: string;
  readonly transform?: Transform;
  /** Clips: keyframes of x, y, scale and opacity (D75, box.ts); ease may name a spring (D192, spring.ts). */
  readonly keys?: ReadonlyArray<{ readonly frame: Frame; readonly ease?: "linear" | "in" | "out" | "inOut" | "spring" | "spring.snap" | "spring.settle" | "spring.land" | "spring.bounce"; readonly x?: number; readonly y?: number; readonly scale?: number; readonly opacity?: number; readonly [key: string]: unknown }>;
  readonly transitionIn?: TimedModule;
  readonly animation?: { readonly in?: TimedModule; readonly out?: TimedModule; readonly loop?: TimedModule };
  readonly effects?: ReadonlyArray<ModuleUse>;
  // clip, component, audio
  readonly media?: Id;
  /** Recorded comments, independently layered and linked to their source video's edit clock. */
  readonly danmaku?: {readonly settings: import('@/app/lib/renders').DanmakuSettings;readonly width:number;readonly height:number};
  readonly in?: Frame;
  // clip, audio: playback rate (D125); keepPitch false lets the pitch follow it.
  readonly speed?: number;
  readonly keepPitch?: boolean;
  /** Clip, audio (D123): makes no sound. Detach audio sets it on the clip whose sound went to an audio item. */
  readonly mute?: boolean;
  /**
   * Audio and off-main clips (D123, D215): the clip this item follows (a detached sound, a take's
   * microphone or other picture under its picture on Main); while set it follows that clip
   * (arrange.ts withLinkedAudio).
   */
  readonly linkedTo?: Id;
  // clip, audio
  readonly gainDb?: number;
  // component
  readonly component?: ModuleId;
  // component, graphic
  readonly props?: { readonly [key: string]: unknown };
  // graphic
  readonly src?: string;
  /** D291: false keeps this item sharp in an export with motion blur on (graphics, for now). */
  readonly motionBlur?: boolean;
  // text
  readonly text?: string;
  // text: TextStyle (the schema lists its fields); captions: a free-form object in the schema,
  // so other keys are allowed.
  readonly style?: TextStyle & { readonly [key: string]: unknown };
  // captions (D82): the source media, a snapshot of its transcript words (seconds of the media,
  // where each is heard), corrections by word index, forced card starts and how cards are grouped.
  readonly source?: Id;
  readonly mode?: "line" | "word" | "single";
  readonly words?: ReadonlyArray<{ readonly text: string; readonly start: number; readonly end: number; readonly filler?: boolean }>;
  readonly edits?: ReadonlyArray<{ readonly word: number; readonly text: string }>;
  readonly breaks?: ReadonlyArray<number>;
  readonly lines?: { readonly maxChars?: number; readonly maxLines?: number; readonly maxSeconds?: number; readonly maxWords?: number; readonly pause?: number };
  readonly hideFillers?: boolean;
  /** Captions (D82): draw wherever the source plays on main, whatever start/duration say (set by Make captions). */
  readonly coversMain?: boolean;
  /** Captions (D215): follow every item that plays the source on any track (a take's microphone off Main), not only the main track's clips. */
  readonly anyTrack?: boolean;
  /** Drawn text only (captions' cards, never saved): transform.y is the box's bottom edge, so wrapping grows upward. */
  readonly anchor?: "bottom";
  // clip, audio
  readonly fadeIn?: Frame;
  readonly fadeOut?: Frame;
  /** The media starts over when the item is longer than it (clip: D75). */
  readonly loop?: boolean;
  /** Clips (D94): the media's last frame stays on screen when the clip is longer than it (a freeze); loop wins. */
  readonly hold?: boolean;
  readonly gainKeys?: ReadonlyArray<{ readonly frame: Frame; readonly db: number }>;
}

export interface Mixer {
  readonly gainDb?: number;
  readonly pan?: number;
  readonly mute?: boolean;
  /** Audio (D123): the clip this sound was detached from; while set it follows that clip (arrange.ts withLinkedAudio). */
  readonly linkedTo?: Id;
  readonly solo?: boolean;
  readonly effects?: ReadonlyArray<ModuleUse>;
}

export interface Track {
  readonly id: Id;
  readonly kind: "video" | "text" | "audio";
  readonly role?: "main";
  /** Subtitle identity is independent of the user-facing, editable track name. */
  readonly purpose?: "subtitles";
  readonly name: string;
  readonly locked?: boolean;
  readonly hidden?: boolean;
  readonly mixer?: Mixer;
  readonly items: ReadonlyArray<Item>;
}

export interface Scene {
  readonly id: Id;
  readonly name: string;
  readonly start: Frame;
  readonly end: Frame;
}

/**
 * Issue #118: a stretch of the timeline the user marked, frames [start, end). "locked": nothing
 * playing there may move or change (applyEdit refuses an edit that would, whoever makes it);
 * "redo": not OK yet, for the agent to redo (`note` says what's wrong).
 */
export interface Section {
  readonly id: Id;
  readonly start: Frame;
  readonly end: Frame;
  readonly state: "locked" | "redo";
  readonly note?: string;
}

export interface Marker {
  readonly frame: Frame;
  readonly label?: string;
}

export interface Project {
  readonly format: string;
  readonly name: string;
  readonly canvas: Canvas;
  readonly theme?: { readonly colors?: Readonly<Record<string, string>>; readonly fonts?: Readonly<Record<string, string>>; readonly margin?: number };
  readonly media: ReadonlyArray<Media>;
  readonly tracks: ReadonlyArray<Track>;
  readonly scenes?: ReadonlyArray<Scene>;
  readonly markers?: ReadonlyArray<Marker>;
  /** Issue #118: locked stretches and ones flagged for the agent to redo. */
  readonly sections?: ReadonlyArray<Section>;
  /** D83: gainDb is the master volume (−60…+12 dB, after the tracks are summed). */
  readonly master?: { readonly gainDb?: number; readonly loudness?: { readonly targetLufs: number }; readonly effects?: ReadonlyArray<ModuleUse> };
  /**
   * D291: motion blur in the export (graphics): `on` its default (absent: on for Standard and High,
   * off for Draft; the Export dialog's switch decides in the end), `samples` sub-frames averaged per
   * frame (2–16, default 4) over `shutter` degrees of the frame interval (1–360, default 90).
   */
  readonly motionBlur?: { readonly on?: boolean; readonly samples?: number; readonly shutter?: number };
  /** D90: taste checks the project turned off (rule ids from core checks.ts). */
  readonly checks?: { readonly disabled?: ReadonlyArray<string>; readonly [key: string]: unknown };
}

/** Fields an op sets; a null value removes the field. */
export type FieldSet = { readonly [key: string]: unknown };

export type Op =
  | { readonly op: "insert_item"; readonly track: Id; readonly index?: number; readonly item: Item }
  | { readonly op: "update_item"; readonly item: Id; readonly set: FieldSet }
  | { readonly op: "trim_item"; readonly item: Id; readonly edge: "start" | "end"; readonly frames: number }
  | { readonly op: "split_item"; readonly item: Id; readonly at: Frame; readonly newId: Id }
  | { readonly op: "move_item"; readonly item: Id; readonly track?: Id; readonly index?: number; readonly start?: Frame }
  | { readonly op: "delete_item"; readonly item: Id }
  | { readonly op: "set_transition"; readonly item: Id; readonly transition: TimedModule | null }
  // The schema only says "an object" for the new track (the project validator checks the result),
  // so every field is optional here. insert_item's item and add_media's media are also plain
  // objects in the schema but are typed as Item and Media: the engine reads their id and type.
  | { readonly op: "add_track"; readonly track: Partial<Track> & { readonly [key: string]: unknown }; readonly index?: number }
  | { readonly op: "update_track"; readonly track: Id; readonly set: FieldSet }
  | { readonly op: "delete_track"; readonly track: Id }
  | { readonly op: "add_media"; readonly media: Media }
  // Another file (a re-shot recording, D98), name, probe or source of a media entry, in place.
  | { readonly op: "update_media"; readonly media: Id; readonly set: FieldSet }
  // F8 (D167): a media entry out of the project (no item may still use it; its file stays on disk).
  | { readonly op: "delete_media"; readonly media: Id }
  | { readonly op: "update_project"; readonly set: FieldSet };

export type OpName = Op["op"];
export type Actor = "user" | "agent";

export interface LogEntry {
  readonly revision: number;
  /** ISO time. */
  readonly at: string;
  readonly actor: Actor;
  readonly ops: ReadonlyArray<Op>;
  readonly turnId?: string;
  readonly summary?: string;
}

/** The open project, its revision and the edits that led there. */
export interface Session {
  readonly project: Project;
  readonly revision: number;
  readonly log: ReadonlyArray<LogEntry>;
}

/** One edit: a list of ops applied atomically by one actor. */
export interface Edit {
  readonly actor: Actor;
  readonly ops: ReadonlyArray<Op>;
  /** The revision the actor last saw; defaults to the session's. */
  readonly baseRevision?: number;
  readonly turnId?: string;
  readonly summary?: string;
  readonly at?: string;
}

/** A validation problem. */
export interface Problem {
  readonly code: string;
  readonly path: string;
  readonly message: string;
}

export type ModuleKind = "transition" | "move" | "text-style" | "component" | "effect" | "audio-processor" | "analyzer" | "generator" | "capture-source";

/** A module manifest (*.json in a modules directory). Kind-specific blocks are left open. */
export interface ModuleManifest {
  readonly manifest: "biliup.module/1";
  readonly id: ModuleId;
  readonly kind: ModuleKind;
  readonly name: string;
  readonly version: string;
  readonly runtime: unknown;
  /** JSON Schema for the module's params or props. */
  readonly params?: object;
  readonly transition?: { readonly minFrames?: number; readonly maxFrames?: number; readonly [key: string]: unknown };
  readonly [key: string]: unknown;
}

export type Modules = ReadonlyMap<ModuleId, ModuleManifest>;
