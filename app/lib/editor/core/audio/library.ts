// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The sound library (D134): online royalty-free music and sound effects (Openverse without a key,
// Freesound with the user's own key) next to Uncut's bundled CC0 sound-effects pack in assets/sfx
// (no bundled music since D274). Browser-safe: the Audio and Sound effects tabs, the server's
// library.* calls and the agent's library_search / library_add all read it.
//
// Whatever is added from the library is downloaded (or copied) into the project's media/ folder
// and carries its credits in the media's `source` (module library.openverse, library.freesound or
// library.bundled): where it came from, its title and creator, its licence and the attribution
// line CC BY asks for (D135: creditsOf / creditsText list them for the video's description).
import sfxJson from "@/resources/editor/sfx/library.json" with { type: "json" };
import type { Media, Project } from "@/app/lib/editor/core/timeline/types";
import { expandQuery, matchesQuery, scoreSound, SOUND_ALIASES } from "@/app/lib/editor/core/audio/search";

export { aliasOf, clipNote, expandQuery, matchesQuery, normalizeText, rankSounds, scoreSound, SOUND_ALIASES, squash, synonymsOf, wordsOf, type ExpandedQuery, type Rankable, type SoundAlias } from "@/app/lib/editor/core/audio/search";

export type LibraryKind = "music" | "sfx";
/** Online sources (D134). */
export type LibrarySource = "openverse" | "freesound";
export const LIBRARY_SOURCES: ReadonlyArray<LibrarySource> = ["openverse", "freesound"];
export const LIBRARY_SOURCE_NAMES: Readonly<Record<LibrarySource, string>> = { openverse: "Openverse", freesound: "Freesound" };

/** Licences the library offers: both allow commercial use and changes. */
export type LibraryLicense = "cc0" | "by";
export const LICENSE_NAMES: Readonly<Record<LibraryLicense, string>> = { cc0: "CC0", by: "CC BY" };

/** Length filter, in the library's words: under 30 s, 30 s to 2 min, over 2 min. */
export type LibraryLength = "short" | "medium" | "long";
export const LIBRARY_LENGTHS: ReadonlyArray<{ readonly id: LibraryLength; readonly name: string }> = [
  { id: "short", name: "Under 30 s" },
  { id: "medium", name: "30 s to 2 min" },
  { id: "long", name: "Over 2 min" },
];

/** A search: words, what kind of sound, and the filters. */
export interface LibraryQuery {
  readonly kind: LibraryKind;
  readonly query: string;
  readonly source?: LibrarySource | undefined;
  readonly license?: LibraryLicense | undefined;
  readonly length?: LibraryLength | undefined;
  /** Music only: full songs (Openverse's music category), not loops and sounds. */
  readonly songs?: boolean | undefined;
  readonly page?: number | undefined;
}

/** One result: what the list shows and what credits need. */
export interface LibraryHit {
  readonly source: LibrarySource;
  /** The source's own id (Openverse's uuid, Freesound's number as text). */
  readonly id: string;
  readonly title: string;
  readonly creator: string;
  readonly creatorUrl?: string | undefined;
  /** The sound's page (Freesound, ...). */
  readonly landingUrl: string;
  /** Where the file comes from (Openverse: the provider that hosts it, e.g. freesound). */
  readonly provider?: string | undefined;
  readonly license: LibraryLicense;
  readonly licenseVersion: string;
  readonly licenseUrl: string;
  /** The line CC BY asks for (Openverse gives one; else made by attributionLine). */
  readonly attribution: string;
  readonly seconds?: number | undefined;
  readonly tags: ReadonlyArray<string>;
  /** The file's type (mp3, ogg, ...), when known. */
  readonly filetype?: string | undefined;
}

/** A hit's id across sources, as the agent and the renderer name it: openverse:<uuid>, freesound:<n>. */
export const hitKey = (hit: Pick<LibraryHit, "source" | "id">) => `${hit.source}:${hit.id}`;
export function parseHitKey(key: string): { readonly source: LibrarySource; readonly id: string } | undefined {
  const m = /^(openverse|freesound):([A-Za-z0-9-]{1,64})$/.exec(key);
  return m ? { source: m[1] as LibrarySource, id: m[2]! } : undefined;
}

/** Quick searches for music: a chip and the words it searches for. */
export const MUSIC_GENRES: ReadonlyArray<{ readonly id: string; readonly name: string; readonly query: string }> = [
  { id: "lofi", name: "Lo-fi", query: "lofi" },
  { id: "ambient", name: "Ambient", query: "ambient" },
  { id: "cinematic", name: "Cinematic", query: "cinematic" },
  { id: "upbeat", name: "Upbeat", query: "upbeat" },
  { id: "piano", name: "Piano", query: "piano" },
  { id: "electronic", name: "Electronic", query: "electronic" },
  { id: "acoustic", name: "Acoustic", query: "acoustic guitar" },
  { id: "hiphop", name: "Hip hop", query: "hip hop" },
];

/**
 * Sound-effect categories: a chip, the words it searches for online (each checked on Openverse to
 * return CC0 / CC BY results, D134 and D208), and the bundled sounds that belong to it. Memes come
 * as sounds of their own (MEME_SOUNDS).
 */
export const SFX_CATEGORIES: ReadonlyArray<{ readonly id: string; readonly name: string; readonly query: string }> = [
  { id: "whoosh", name: "Whoosh", query: "whoosh" },
  { id: "transition", name: "Transitions", query: "swoosh transition" },
  { id: "swipe", name: "Swipes", query: "swipe" },
  { id: "ui", name: "UI clicks", query: "ui click" },
  { id: "pop", name: "Pops", query: "pop" },
  { id: "impact", name: "Impacts", query: "impact" },
  { id: "riser", name: "Risers", query: "riser" },
  { id: "notification", name: "Notifications", query: "notification" },
  { id: "success", name: "Success", query: "success" },
  { id: "glitch", name: "Glitch", query: "glitch" },
  { id: "typing", name: "Typing", query: "keyboard typing" },
  { id: "crowd", name: "Laughs and applause", query: "applause" },
  { id: "meme", name: "Memes", query: "bass boom" },
];

/** Meme sounds by what people call them (SOUND_ALIASES with a chip), mapped to searches that find CC0 / CC BY sound-alikes. */
export const MEME_SOUNDS: ReadonlyArray<{ readonly id: string; readonly name: string; readonly query: string }> = SOUND_ALIASES.filter((a) => a.chip).map((a) => ({ id: a.id, name: a.name, query: a.online }));

/** A bundled sound effect (assets/sfx/library.json). */
export interface SfxSound {
  readonly id: string;
  readonly file: string;
  readonly title: string;
  readonly category: string;
  readonly tags: ReadonlyArray<string>;
  readonly seconds: number;
  readonly bytes: number;
  readonly trimmed?: string;
  /** Integrated loudness and sample peak after levelling (make-sfx). */
  readonly lufs: number;
  readonly peakDb: number;
  /** 48 peaks, 0 to 100, for the list's waveform. */
  readonly peaks: ReadonlyArray<number>;
  readonly author: string;
  /**
   * Where it came from: a Freesound sound page, a Kenney pack page (with the file in the pack), or
   * Uncut's own synthesiser (scripts/sfx-synth.ts, D250: made from nothing, so CC0 by Uncut).
   */
  readonly provider: "freesound" | "kenney" | "uncut";
  readonly source: string;
  readonly sourceFile?: string;
  readonly license: string;
  readonly licenseChecked: string;
  /** How the licence was checked. */
  readonly licenseProof: string;
}

export const SFX_SOUNDS: ReadonlyArray<SfxSound> = (sfxJson as { sounds: ReadonlyArray<SfxSound> }).sounds;
export const sfxSound = (id: string) => SFX_SOUNDS.find((s) => s.id === id);
/** Meme and crowd sounds: only when the user asks for that joke (D207). */
export const onlyWhenAsked = (s: Pick<SfxSound, "category">) => s.category === "meme" || s.category === "crowd";

/** Bundled sounds matching words (spellings, synonyms and meme names too, D208) and a category, best first; all when neither is given. */
export function filterSfx({ query, category }: { readonly query?: string | undefined; readonly category?: string | undefined } = {}): Array<SfxSound> {
  const q = expandQuery(query ?? "");
  const found = SFX_SOUNDS.filter((s) => {
    if (category && s.category !== category) return false;
    const categoryName = SFX_CATEGORIES.find((c) => c.id === s.category)?.name ?? "";
    return matchesQuery({ id: s.id, title: s.title, tags: s.tags, category: s.category, extra: `${s.author} ${categoryName}` }, q);
  });
  if (!q.groups.length) return found;
  const scored = found.map((s, i) => ({ s, i, score: (q.alias?.bundled.includes(s.id) ? 20 - 5 * q.alias.bundled.indexOf(s.id) : 0) + scoreSound({ ...s, license: "cc0" }, q) }));
  return scored.sort((a, b) => b.score - a.score || a.i - b.i).map((e) => e.s);
}

/** The modules a library media's source carries. */
export const LIBRARY_MODULES = { openverse: "library.openverse", freesound: "library.freesound", bundled: "library.bundled", folder: "library.folder" } as const;

/** What a media item added from the library records about itself (its `source`). */
export interface LibraryCredit {
  readonly module: string;
  /** hitKey for online sounds, sfx:<id> for bundled ones. */
  readonly libraryId: string;
  readonly title: string;
  readonly creator: string;
  readonly creatorUrl?: string;
  readonly landingUrl: string;
  readonly provider?: string;
  readonly fileUrl?: string;
  readonly license: LibraryLicense;
  readonly licenseVersion: string;
  readonly licenseUrl: string;
  readonly attribution: string;
  readonly sha256?: string;
  /** When it was downloaded (ISO 8601). */
  readonly downloaded?: string;
}

const LICENSE_URLS: Readonly<Record<LibraryLicense, (version: string) => string>> = {
  cc0: () => "https://creativecommons.org/publicdomain/zero/1.0/",
  by: (v) => `https://creativecommons.org/licenses/by/${v}/`,
};
export const licenseUrl = (license: LibraryLicense, version: string) => LICENSE_URLS[license](version);

/** The attribution line, Openverse's wording: "Title" by Creator is licensed under CC BY 4.0. To view ... */
export function attributionLine(hit: Pick<LibraryHit, "title" | "creator" | "license" | "licenseVersion" | "licenseUrl">) {
  const verb = hit.license === "cc0" ? "is marked with CC0" : "is licensed under CC BY";
  return `"${hit.title}" by ${hit.creator} ${verb} ${hit.licenseVersion}. To view the terms, visit ${hit.licenseUrl}`;
}

/** The source a hit's media gets once downloaded: its credits. */
export function creditOf(hit: LibraryHit, extra: { readonly fileUrl?: string; readonly sha256?: string; readonly downloaded?: string } = {}): LibraryCredit {
  return {
    module: LIBRARY_MODULES[hit.source],
    libraryId: hitKey(hit),
    title: hit.title,
    creator: hit.creator,
    ...(hit.creatorUrl ? { creatorUrl: hit.creatorUrl } : {}),
    landingUrl: hit.landingUrl,
    ...(hit.provider ? { provider: hit.provider } : {}),
    ...(extra.fileUrl ? { fileUrl: extra.fileUrl } : {}),
    license: hit.license,
    licenseVersion: hit.licenseVersion,
    licenseUrl: hit.licenseUrl,
    attribution: hit.attribution,
    ...(extra.sha256 ? { sha256: extra.sha256 } : {}),
    ...(extra.downloaded ? { downloaded: extra.downloaded } : {}),
  };
}

/** The source a bundled sound effect's media gets. */
export function sfxCredit(s: SfxSound): LibraryCredit {
  const hit = { title: s.title, creator: s.author, license: "cc0" as const, licenseVersion: "1.0", licenseUrl: licenseUrl("cc0", "1.0") };
  return { module: LIBRARY_MODULES.bundled, libraryId: `sfx:${s.id}`, ...hit, landingUrl: s.source, provider: s.provider, attribution: attributionLine(hit) };
}

/** The media in a project that came from the library entry `libraryId` (re-adding it reuses it). */
export const libraryMediaOf = (project: Pick<Project, "media">, libraryId: string) => project.media.find((m) => m.source?.libraryId === libraryId);

/** Attribution stored on media imported from an audio library. */
export function mediaCredit(media: Pick<Media, "path" | "source">): LibraryCredit | undefined {
  const s = media.source as Partial<LibraryCredit> | undefined;
  if (s && typeof s.module === "string" && s.module.startsWith("library.") && typeof s.title === "string" && (s.license === "cc0" || s.license === "by")) return s as LibraryCredit;
  return undefined;
}

export interface Credits {
  /** CC BY: the credit each one needs, in the order they were added. */
  readonly attribution: ReadonlyArray<LibraryCredit>;
  /** CC0: no credit needed (listed anyway, as thanks). */
  readonly publicDomain: ReadonlyArray<LibraryCredit>;
  /** Some came through Openverse (its terms ask to say so). */
  readonly openverse: boolean;
}

/** The project's library credits (D135), one per media item (duplicates of one sound once). */
export function creditsOf(project: Pick<Project, "media">): Credits {
  const seen = new Set<string>();
  const attribution: Array<LibraryCredit> = [];
  const publicDomain: Array<LibraryCredit> = [];
  let openverse = false;
  for (const m of project.media) {
    const c = mediaCredit(m);
    if (!c || seen.has(c.libraryId)) continue;
    seen.add(c.libraryId);
    if (c.module === LIBRARY_MODULES.openverse) openverse = true;
    (c.license === "by" ? attribution : publicDomain).push(c);
  }
  return { attribution, publicDomain, openverse };
}

/** The credits as text for a video's description; empty when nothing came from the library. */
export function creditsText(project: Pick<Project, "media" | "name">): string {
  const c = creditsOf(project);
  if (!c.attribution.length && !c.publicDomain.length) return "";
  const lines: Array<string> = [];
  if (c.attribution.length) {
    lines.push("Music and sound");
    for (const a of c.attribution) lines.push(`"${a.title}" by ${a.creator} (${a.landingUrl}), licensed under CC BY ${a.licenseVersion} (${a.licenseUrl})`);
  }
  if (c.publicDomain.length) {
    if (lines.length) lines.push("");
    lines.push("Public domain (CC0)");
    for (const a of c.publicDomain) lines.push(`"${a.title}" by ${a.creator} (${a.landingUrl})`);
  }
  if (c.openverse) lines.push("", "Found with Openverse (openverse.org)");
  return `${lines.join("\n")}\n`;
}

/** The bundled sound effect a media path is (media/<file> or a numbered copy). */
export function bundledSfxOf(media: Pick<Media, "path">): SfxSound | undefined {
  const path = media.path.replace(/\\/g, "/");
  if (!path.startsWith("media/")) return undefined;
  const name = path.slice("media/".length);
  return SFX_SOUNDS.find((s) => s.file === name) ?? SFX_SOUNDS.find((s) => new RegExp(`^${s.file.replace(/\.m4a$/, "")}-\\d+\\.m4a$`).test(name));
}

/** All bundled audio files (the sound effects), for the install check and notices. */
export const BUNDLED_AUDIO_FILES = SFX_SOUNDS.map((s) => `sfx/${s.file}`);

/** "Openverse's search limit is reached. Try again in 40 s." (a limit: its scope and until when, ISO 8601). */
export function limitSentence(source: LibrarySource, limit: { readonly scope: "minute" | "day"; readonly until: string }, now = Date.now()) {
  const wait = Math.max(1, Math.ceil((Date.parse(limit.until) - now) / 1000));
  const when = wait < 90 ? `in ${wait} s` : wait < 5400 ? `in ${Math.ceil(wait / 60)} min` : "tomorrow";
  return `${LIBRARY_SOURCE_NAMES[source]}'s ${limit.scope === "day" ? "daily " : ""}search limit is reached. Try again ${when}.`;
}

/** A sound's length for a list: "0.7 s" under 10 s, else "1:56". */
export const lengthLabel = (seconds: number) => (seconds < 10 ? `${seconds.toFixed(1)} s` : `${Math.floor(seconds / 60)}:${String(Math.round(seconds % 60)).padStart(2, "0")}`);

// ---- the user's sound folders (D208) ----

/** A registered audio asset in a shared, uploaded folder. */
export interface FolderSound {
  /** Hosted folder ID and immutable registered asset ID. */
  readonly id: string;
  /** The folder's id. */
  readonly folder: string;
  /** The file's name without its extension, as the list shows it. */
  readonly name: string;
  /** Its path inside the folder (subfolders are tags). */
  readonly rel: string;
  readonly ext: string;
  readonly bytes: number;
  readonly seconds?: number;
}

/** A folder of the user's sounds. */
export interface SoundFolder {
  readonly id: string;
  readonly name: string;
  readonly version: number;
  readonly sounds: ReadonlyArray<FolderSound>;
}

/** Audio files a folder's index lists. */
export const FOLDER_AUDIO_EXTENSIONS: ReadonlyArray<string> = ["wav", "mp3", "m4a", "aac", "aif", "aiff", "flac", "ogg", "oga", "opus", "caf"];

/** Include the chosen root folder as well as nested folders and the file type. */
const folderWords = (s: FolderSound & { readonly folderName: string }) => `${s.folderName} ${s.rel.split("/").slice(0, -1).join(" ")} ${s.ext}`;

/** The folders' sounds matching words (spellings, synonyms and meme names too), best first; all when no words. */
export function filterFolderSounds(folders: ReadonlyArray<SoundFolder>, query = "", folder?: string): Array<FolderSound & { readonly folderName: string }> {
  const q = expandQuery(query);
  const all = folders.filter((f) => !folder || f.id === folder).flatMap((f) => f.sounds.map((s) => ({ ...s, folderName: f.name })));
  if (!q.groups.length) return all;
  return all
    .filter((s) => matchesQuery({ title: s.name, extra: folderWords(s) }, q))
    .map((s, i) => ({ s, i, score: scoreSound({ title: s.name, tags: folderWords(s).split(" ") }, q) }))
    .sort((a, b) => b.score - a.score || a.i - b.i)
    .map((e) => e.s);
}
