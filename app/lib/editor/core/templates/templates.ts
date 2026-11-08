// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Templates (issue #111): creative briefs the user picks in the composer's / menu. A template is a
// Markdown file with a small front matter and placeholders; Biliup fills the placeholders from the
// open project, deterministically, and the filled brief lands in the composer for the user to edit
// and send. Built-ins ship with Biliup (builtin.ts); the user's own are .md files in the app's data
// folder (<dataDir>/templates/<id>.md), made from scratch or by duplicating a built-in.
//
// The file:
//
//   ---
//   name: Beat-cut reel
//   description: One line on what it makes.
//   ---
//   The brief, in Markdown, with placeholders.
//
// Placeholders: {{name}} or {{name|fallback}}. Names are lowercase (TEMPLATE_KEYS lists them and
// what each becomes). A placeholder with no value in this project becomes its fallback when it has
// one, else an honest bracketed note ("[no beat analysis yet …]") that says what's missing, never
// a guess. An unknown name is left as written, so a typo shows. \{{ writes a literal {{.
//
// Browser-safe and pure: the server fills, the web lists and edits.

/** What each placeholder becomes (documented for the template editor and the tests). */
export const TEMPLATE_KEYS = {
  project: "The project's name.",
  product: "The product's name: the brand kit's name.",
  media: "Every file in the media library, one line each: name, kind, length, fps, size, sound.",
  footage: "The video and image files only, one line each.",
  "footage-kind": "What the footage is, in a user's words: drone footage when every clip is named as drone or aerial footage, else footage.",
  music: "The song: the longest audio file (or the analysed one), its length, tempo and drop when known.",
  tempo: "The song's tempo from its beat analysis, as measured (e.g. 121.8 BPM).",
  drop: "Where the song's drop lands (the beat where the kick and bass step up), from its beat analysis.",
  beats: "How many beats the song has over the film's length, from its beat analysis.",
  kit: "The project's brand kit: its name, logo and font files, colours, other files, its about and tone lines, taglines and URLs.",
  palette: "The kit's colours grouped by role, one role per line (background, surface, text, accent ...).",
  fonts: "The kit's typefaces with what each is for and its file (e.g. DM Sans for heading and body (DMSans.woff2)).",
  logos: "The kit's logo files and where each goes (on dark, on light, the mark).",
  screens: "The product's real UI screenshots: the kit's example files, else the images whose names say ui or screen, one line each.",
  look: "The kit's dos, one line each: how the product looks and what to show.",
  avoid: "The kit's don'ts, one line each.",
  tagline: "The kit's first tagline.",
  url: "The kit's first URL.",
  grid: "The song's beat grid from its beat analysis: beat and bar length, first downbeat, bars over the film's length, the drop's bar.",
  canvas: "The canvas: size, aspect ratio and frame rate (e.g. 1080 x 1350 (4:5), 60 fps).",
  length: "The film's length: the brief's duration, else the timeline's, else the song's.",
} as const;
export type TemplateKey = keyof typeof TEMPLATE_KEYS;
export const isTemplateKey = (k: string): k is TemplateKey => Object.hasOwn(TEMPLATE_KEYS, k);

/** One line for the editor: the syntax and the names. */
export const TEMPLATE_SYNTAX = `Placeholders: ${Object.keys(TEMPLATE_KEYS)
  .map((k) => `{{${k}}}`)
  .join(" ")}. {{name|fallback}} uses the fallback when the project has no value; \\{{ is a literal {{.`;

/** Ids of templates: lowercase letters, digits and dashes (also the file name, <id>.md). */
export const TEMPLATE_ID = /^[a-z0-9][a-z0-9-]{0,63}$/;
export const TEMPLATE_BODY_MAX = 40000;

export interface Template {
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly body: string;
  /** Ships with Biliup: read-only, duplicate it to change it. */
  readonly builtin?: true;
}

/** A template file's text as a template; the id comes from its file name. Lenient: no front matter, the body is the whole text and the name is the id. */
export function parseTemplate(text: string, id: string): Template {
  const src = text.replace(/^﻿/, "").replace(/\r\n?/g, "\n");
  const m = /^---\n([\s\S]*?)\n---[ \t]*(?:\n|$)/.exec(src);
  const meta: Record<string, string> = {};
  if (m)
    for (const line of m[1]!.split("\n")) {
      const kv = /^\s*([a-z]+)\s*:\s*(.*?)\s*$/i.exec(line);
      if (kv) meta[kv[1]!.toLowerCase()] = unquote(kv[2]!);
    }
  const body = (m ? src.slice(m[0].length) : src).replace(/^\n+/, "").slice(0, TEMPLATE_BODY_MAX);
  return { id, name: (meta.name ?? "").slice(0, 80).trim() || id, description: (meta.description ?? "").slice(0, 300).trim(), body };
}
const unquote = (v: string) => (v.length >= 2 && (v[0] === '"' || v[0] === "'") && v.at(-1) === v[0] ? v.slice(1, -1) : v);

/** A template as its file's text (what parseTemplate reads back). */
export function formatTemplate(t: Pick<Template, "name" | "description" | "body">): string {
  const one = (s: string) => s.replace(/\s*\n\s*/g, " ").trim();
  return `---\nname: ${one(t.name)}\ndescription: ${one(t.description)}\n---\n\n${t.body.replace(/^\n+/, "").replace(/\s*$/, "\n")}`;
}

/** A free id for a template named `name` (slug of the name; -2, -3 … when taken). */
export function templateId(name: string, taken: ReadonlyArray<string>): string {
  const base =
    name
      .toLowerCase()
      .normalize("NFKD")
      .replace(/[̀-ͯ]/g, "")
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 48) || "template";
  const used = new Set(taken);
  if (!used.has(base)) return base;
  for (let n = 2; ; n++) if (!used.has(`${base}-${n}`)) return `${base}-${n}`;
}

// ---- the project's facts ----

export interface TemplateMedia {
  readonly name: string;
  readonly kind: "video" | "image" | "audio";
  readonly seconds?: number | undefined;
  readonly fps?: number | undefined;
  readonly width?: number | undefined;
  readonly height?: number | undefined;
  readonly hasAudio?: boolean | undefined;
}

/** What the server knows about the open project, as plain data (templateValues turns it into text). */
export interface TemplateFacts {
  readonly project?: string | undefined;
  readonly media: ReadonlyArray<TemplateMedia>;
  readonly canvas?: { readonly width: number; readonly height: number; readonly fps: number } | undefined;
  /** The film's length in seconds and where it came from. */
  readonly length?: { readonly seconds: number; readonly from: "brief" | "timeline" | "song" } | undefined;
  readonly kit?:
    | {
        readonly name: string;
        readonly logos: ReadonlyArray<string>;
        readonly fonts: ReadonlyArray<string>;
        readonly files: ReadonlyArray<string>;
        readonly colors: ReadonlyArray<{ readonly hex: string; readonly role: string; readonly name?: string | undefined }>;
        readonly about?: string | undefined;
        readonly tone?: string | undefined;
        /** Which logo file goes where. */
        readonly logoRoles?: { readonly dark?: string | undefined; readonly light?: string | undefined; readonly mark?: string | undefined } | undefined;
        readonly typefaces?: ReadonlyArray<{ readonly family: string; readonly roles: ReadonlyArray<string>; readonly file?: string | undefined }> | undefined;
        /** The product's UI screenshots in the kit. */
        readonly screens?: ReadonlyArray<string> | undefined;
        readonly taglines?: ReadonlyArray<string> | undefined;
        readonly urls?: ReadonlyArray<string> | undefined;
        readonly dos?: ReadonlyArray<string> | undefined;
        readonly donts?: ReadonlyArray<string> | undefined;
      }
    | undefined;
  /** The song and, when it has been analysed, its tempo and drop (seconds of the song). */
  readonly music?:
    | {
        readonly name: string;
        readonly seconds?: number | undefined;
        /** undefined: never analysed; null: analysed, no steady beat. */
        readonly bpm?: number | null | undefined;
        readonly confidence?: number | undefined;
        /** The analysed beats inside the first beatSpan seconds (the film's length, else the whole song). */
        readonly beatCount?: number | undefined;
        readonly beatSpan?: number | undefined;
        readonly drop?: { readonly seconds: number; readonly beat: number } | undefined;
        /** The first downbeat (seconds of the song) and its beat's index: bars count from it. */
        readonly downbeat?: { readonly seconds: number; readonly beat: number } | undefined;
      }
    | undefined;
}

/** Seconds as people say them: 8.5 s, 28 s, 1:05. */
export function sayDuration(s: number): string {
  if (!Number.isFinite(s) || s < 0) return "";
  if (s < 60) return `${Math.round(s * 10) / 10} s`;
  const whole = Math.round(s);
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, "0")}`;
}
/** A moment of a song: 0:14.2. */
const sayMoment = (s: number) => {
  const m = Math.floor(s / 60);
  const rest = Math.round((s - m * 60) * 10) / 10;
  return `${m}:${rest < 10 ? "0" : ""}${rest.toFixed(1)}`;
};
const gcd = (a: number, b: number): number => (b ? gcd(b, a % b) : a);
/** 1080 x 1350 → 4:5; odd sizes keep their numbers. */
export function aspectOf(width: number, height: number): string {
  const g = gcd(width, height) || 1;
  const a = width / g;
  const b = height / g;
  return a <= 64 && b <= 64 ? `${a}:${b}` : `${Math.round((width / height) * 100) / 100}:1`;
}
const fpsWord = (fps: number) => `${Math.round(fps * 1000) / 1000} fps`;

function mediaLine(m: TemplateMedia): string {
  const bits: Array<string> = [m.kind === "audio" ? "audio" : m.kind];
  if (m.kind !== "image" && m.seconds !== undefined) bits.push(sayDuration(m.seconds));
  if (m.kind === "video" && m.fps) bits.push(fpsWord(m.fps));
  if (m.kind !== "audio" && m.width && m.height) bits.push(`${m.width} x ${m.height}`);
  if (m.kind === "video" && m.hasAudio !== undefined) bits.push(m.hasAudio ? "with sound" : "no sound");
  return `- ${m.name}: ${bits.join(", ")}`;
}

const LIST_MAX = 60;
const listOf = (ms: ReadonlyArray<TemplateMedia>) => {
  const lines = ms.slice(0, LIST_MAX).map(mediaLine);
  if (ms.length > LIST_MAX) lines.push(`- and ${ms.length - LIST_MAX} more`);
  return lines.join("\n");
};

const tempoWord = (bpm: number) => `${Math.round(bpm * 10) / 10} BPM`;
const oneLine = (s: string) => s.trim().replace(/\s*\n\s*/g, " ");
const andList = (xs: ReadonlyArray<string>) => (xs.length < 2 ? xs.join("") : `${xs.slice(0, -1).join(", ")} and ${xs.at(-1)}`);
const IMAGE_FILE = /\.(?:png|jpe?g|webp|gif|avif)$/i;
const SCREEN_NAME = /(?:^|[^a-z])(?:ui|screen|screenshot|app)(?:[^a-z]|$)/i;
/** Clip names that say the clip was shot from a drone (DJI_ alone doesn't: DJI makes handheld cameras too). */
const DRONE_NAME = /drone|aerial|fpv|mavic/i;
const secondsWord = (s: number) => `${Math.round(s * 100) / 100} s`;

/** The kit's colours, one role per line in the order the roles first appear: "- surface: #111111 (panel), #161616". */
function paletteOf(colors: NonNullable<TemplateFacts["kit"]>["colors"]): string {
  const roles = new Map<string, Array<string>>();
  for (const c of colors) {
    const list = roles.get(c.role) ?? [];
    list.push(c.name && c.name !== c.role ? `${c.hex} (${c.name})` : c.hex);
    roles.set(c.role, list);
  }
  return [...roles].map(([role, hexes]) => `- ${role}: ${hexes.join(", ")}`).join("\n");
}

/** The beat grid in words: "120 BPM: a beat every 0.5 s, a bar of 4 beats every 2 s; first downbeat at 0:00.5; 14 bars in 28 s; the drop on bar 7, at 0:12.5". */
function gridOf(bpm: number, s: NonNullable<TemplateFacts["music"]>): string {
  const beat = 60 / bpm;
  const parts = [`${tempoWord(bpm)}: a beat every ${secondsWord(beat)}, a bar of 4 beats every ${secondsWord(beat * 4)}`];
  // Bars count from the first downbeat; without one, the drop's phase (the drop is a downbeat).
  const first = s.downbeat ?? (s.drop ? { seconds: s.drop.seconds - (s.drop.beat - (s.drop.beat % 4)) * beat, beat: s.drop.beat % 4 } : undefined);
  if (first) parts.push(`the first downbeat at ${sayMoment(Math.max(0, first.seconds))} into the song`);
  const span = s.beatSpan ?? s.seconds;
  if (s.beatCount && span !== undefined) parts.push(`${s.beatCount} beats (${Math.floor(s.beatCount / 4)} bars) in ${sayDuration(span)}`);
  if (s.drop) parts.push(`the drop on bar ${Math.floor((s.drop.beat - (first?.beat ?? 0)) / 4) + 1}, at ${sayMoment(s.drop.seconds)}`);
  return parts.join("; ");
}

/** The placeholders' values for these facts; a key is missing when the project has nothing true to say. */
export function templateValues(f: TemplateFacts): Partial<Record<TemplateKey, string>> {
  const out: Partial<Record<TemplateKey, string>> = {};
  if (f.project?.trim()) out.project = f.project.trim();
  if (f.media.length) out.media = listOf(f.media);
  const footage = f.media.filter((m) => m.kind !== "audio");
  if (footage.length) out.footage = listOf(footage);
  const clips = footage.filter((m) => m.kind === "video");
  if (footage.length) out["footage-kind"] = clips.length && clips.every((m) => DRONE_NAME.test(m.name)) ? "drone footage" : "footage";
  if (f.canvas) out.canvas = `${f.canvas.width} x ${f.canvas.height} (${aspectOf(f.canvas.width, f.canvas.height)}), ${fpsWord(f.canvas.fps)}`;
  if (f.length && f.length.seconds > 0) out.length = sayDuration(f.length.seconds);
  const k = f.kit;
  if (k) {
    const lines = [`"${k.name}" brand kit`];
    if (k.logos.length) lines.push(`- Logos (use these files, never redraw them): ${k.logos.join(", ")}`);
    if (k.fonts.length) lines.push(`- Fonts: ${k.fonts.join(", ")}`);
    if (k.colors.length) lines.push(`- Colours: ${k.colors.map((c) => `${c.hex} ${c.name ? `${c.name} (${c.role})` : c.role}`).join(", ")}`);
    if (k.files.length) lines.push(`- Other files: ${k.files.join(", ")}`);
    if (k.about?.trim()) lines.push(`- About: ${oneLine(k.about)}`);
    if (k.tone?.trim()) lines.push(`- Tone: ${oneLine(k.tone)}`);
    const taglines = (k.taglines ?? []).map(oneLine).filter(Boolean);
    const urls = (k.urls ?? []).map(oneLine).filter(Boolean);
    if (taglines.length) lines.push(`- Taglines: ${taglines.map((t) => `"${t}"`).join(", ")}`);
    if (urls.length) lines.push(`- URLs: ${urls.join(", ")}`);
    out.kit = lines.join("\n");
    if (k.name.trim()) out.product = oneLine(k.name);
    if (k.colors.length) out.palette = paletteOf(k.colors);
    const faces = k.typefaces?.length ? k.typefaces : undefined;
    if (faces) out.fonts = faces.map((f) => `${f.family}${f.roles.length ? ` for ${andList(f.roles)}` : ""} (${f.file ?? "not in the kit as a file"})`).join("; ");
    else if (k.fonts.length) out.fonts = [...new Set(k.fonts)].join("; ");
    const r = k.logoRoles;
    const placed = r ? [r.dark && `${r.dark} (on dark)`, r.light && `${r.light} (on light)`, r.mark && `${r.mark} (the mark)`].filter((x): x is string => !!x) : [];
    if (placed.length) out.logos = placed.join(", ");
    else if (k.logos.length) out.logos = k.logos.join(", ");
    const screens = k.screens?.length ? k.screens : k.files.filter((f) => IMAGE_FILE.test(f) && SCREEN_NAME.test(f));
    if (screens.length) out.screens = screens.map((f) => `- ${f}`).join("\n");
    const bullets = (xs: ReadonlyArray<string> | undefined) => (xs ?? []).map(oneLine).filter(Boolean).map((x) => `- ${x}`).join("\n");
    if (bullets(k.dos)) out.look = bullets(k.dos);
    if (bullets(k.donts)) out.avoid = bullets(k.donts);
    if (taglines.length) out.tagline = `"${taglines[0]}"`;
    if (urls.length) out.url = urls[0]!;
  }
  const s = f.music;
  if (s) {
    const bits = [s.name];
    if (s.seconds !== undefined) bits.push(sayDuration(s.seconds));
    if (typeof s.bpm === "number") bits.push(tempoWord(s.bpm));
    else if (s.bpm === null) bits.push("no steady beat found");
    if (s.drop) bits.push(`drop at ${sayMoment(s.drop.seconds)}`);
    out.music = bits.join(", ");
    if (typeof s.bpm === "number") {
      out.tempo = tempoWord(s.bpm);
      const span = s.beatSpan ?? s.seconds;
      if (s.beatCount) out.beats = `${s.beatCount} beats${span !== undefined ? ` in ${sayDuration(span)}` : ""}`;
      if (s.drop) out.drop = `${sayMoment(s.drop.seconds)} into the song (beat ${s.drop.beat + 1})`;
      out.grid = gridOf(s.bpm, s);
    }
  }
  return out;
}

/** What a placeholder says when the project has no value for it and the template gives no fallback. */
export const TEMPLATE_MISSING: Readonly<Record<TemplateKey, string>> = {
  project: "[the project's name]",
  product: "[the product's name]",
  media: "[no media imported yet: import your footage, screenshots and song]",
  footage: "[no footage or images imported yet]",
  "footage-kind": "footage",
  music: "[no song imported yet]",
  tempo: "[tempo not measured yet: analyse the song's beats first (media_analyze with the beats analyzer) and use the tempo it finds]",
  drop: "[drop not known yet: find it in the song's beat analysis (media_analyze with the beats analyzer); if the song has no clear drop, use its strongest downbeat]",
  beats: "[beat count not known yet: analyse the song's beats first (media_analyze with the beats analyzer)]",
  kit: "[no brand kit picked for this project]",
  canvas: "[the canvas size and frame rate]",
  length: "[length not set yet]",
  palette: "[no brand kit colours: take the colours from the screenshots]",
  fonts: "[no fonts in the kit: match the screenshots' type]",
  logos: "[no logo files in the kit]",
  screens: "[no UI screenshots in the kit: add some to the brand kit so the interface can be rebuilt from them]",
  look: "[the kit has no notes on the interface: take radii, borders, spacing and labels from the screenshots]",
  avoid: "[the kit lists no don'ts]",
  tagline: "[no tagline in the kit]",
  url: "[no URL in the kit]",
  grid: "[no beat analysis of the song yet: analyse its beats first (media_analyze with the beats analyzer) and build the grid from the measured tempo, downbeats and drop]",
};

const PLACEHOLDER = /\\\{\{|\{\{\s*([a-z][a-z0-9-]*)\s*(?:\|([^{}]*))?\}\}/g;

/** The placeholders a template uses (known ones, in order, once each). */
export function templatePlaceholders(body: string): Array<TemplateKey> {
  const out = new Set<TemplateKey>();
  for (const m of body.matchAll(PLACEHOLDER)) if (m[1] && isTemplateKey(m[1])) out.add(m[1]);
  return [...out];
}

/**
 * The body with its placeholders filled: a value, else the placeholder's fallback, else its
 * TEMPLATE_MISSING note. `missing`: the known placeholders this project had no value for.
 */
export function fillTemplate(body: string, values: Partial<Record<TemplateKey, string>>): { text: string; missing: Array<TemplateKey> } {
  const missing = new Set<TemplateKey>();
  const text = body.replace(PLACEHOLDER, (whole, name: string | undefined, fallback: string | undefined) => {
    if (!name) return "{{";
    if (!isTemplateKey(name)) return whole;
    const v = values[name];
    if (v !== undefined && v !== "") return v;
    missing.add(name);
    return fallback !== undefined ? fallback.trim() : TEMPLATE_MISSING[name];
  });
  return { text, missing: [...missing] };
}

/** A filled brief put into a composer draft at `at`: alone on its own lines when the draft has text around it. */
export function insertBrief(draft: string, brief: string, at: number): string {
  const i = Math.max(0, Math.min(draft.length, at));
  const head = draft.slice(0, i).replace(/[ \t]+$/, "");
  const tail = draft.slice(i).replace(/^[ \t]+/, "");
  const before = head && !head.endsWith("\n") ? `${head}\n\n` : head;
  const after = tail && !tail.startsWith("\n") ? `\n\n${tail}` : tail;
  return `${before}${brief}${after}`;
}
