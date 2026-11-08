// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The video's type and what follows from it (D217, issues #42, #44, #45): the critic's rubric (each
// category required, optional or not applicable), which blocker kinds fail a round, and how strict the
// sound rule is. A launch promo is judged as before D207; a tutorial needs no hook, a talk no motion
// graphics, a vlog or a meme no brand, and a meme or gaming edit may want dense, loud sound effects.
// A brief written before D217 has no type and keeps the old, strict judgement.
//
// Browser-safe and pure: the server (brief_write, review_submit, project_check) and the Brief and
// review cards read it, so the card and the critic never disagree.
import type { SoundProfile } from "@/app/lib/editor/core/checks/checks";

/** The kinds of video a brief can record, in the order the Brief card offers them. */
export const VIDEO_TYPES = ["launch-promo", "product-demo", "tutorial", "talk", "vlog", "social", "explainer", "music-video", "meme", "gaming", "other"] as const;
export type VideoType = (typeof VIDEO_TYPES)[number];
export const VIDEO_TYPE_NAMES: Readonly<Record<VideoType, string>> = {
  "launch-promo": "Launch promo",
  "product-demo": "Product demo",
  tutorial: "Tutorial",
  talk: "Talk",
  vlog: "Vlog",
  social: "Social cut-down",
  explainer: "Explainer",
  "music-video": "Music video",
  meme: "Meme edit",
  gaming: "Gaming clip",
  other: "Other",
};
export const isVideoType = (v: unknown): v is VideoType => typeof v === "string" && (VIDEO_TYPES as ReadonlyArray<string>).includes(v);

/** The critic's categories (contracts REVIEW_SCORES, repeated so core stays free of contracts). */
export const RUBRIC_CATEGORIES = ["hook", "readability", "motion", "variety", "sound", "brand"] as const;
export type RubricCategory = (typeof RUBRIC_CATEGORIES)[number];
export type RubricLevel = "required" | "optional" | "na";

/** A required category passes at this score, an optional one at this; na isn't scored. */
export const REQUIRED_BAR = 8;
export const OPTIONAL_BAR = 6;

/** Blocker kinds that are real defects whatever the video: they always fail a round. */
export const DEFECT_KINDS = ["placeholder", "missing-asset", "font", "overflow", "illegible", "invented", "blank", "off-centre", "unverified-fix"] as const;

const R = "required" as const;
const O = "optional" as const;
const N = "na" as const;
// hook, readability, motion, variety, sound, brand
const TABLE: Readonly<Record<VideoType, readonly [RubricLevel, RubricLevel, RubricLevel, RubricLevel, RubricLevel, RubricLevel]>> = {
  "launch-promo": [R, R, R, R, R, R],
  "product-demo": [O, R, O, O, O, R],
  tutorial: [N, R, O, O, O, O],
  talk: [N, R, N, O, O, O],
  vlog: [O, R, O, O, O, N],
  social: [R, R, O, R, O, O],
  explainer: [O, R, R, O, O, O],
  "music-video": [O, O, R, R, R, N],
  meme: [R, O, O, O, O, N],
  gaming: [O, O, O, R, O, N],
  other: [O, R, O, O, O, O],
};
const SOUND_BY_TYPE: Readonly<Record<VideoType, SoundProfile>> = {
  "launch-promo": "strict",
  "product-demo": "strict",
  tutorial: "standard",
  talk: "standard",
  vlog: "standard",
  social: "loose",
  explainer: "standard",
  "music-video": "loose",
  meme: "loose",
  gaming: "loose",
  other: "standard",
};

/** What the rubric reads of a brief. */
export interface RubricBrief {
  readonly film?: string;
  readonly visual?: string;
  readonly notes?: string;
  readonly hardLines?: ReadonlyArray<string>;
  readonly type?: string;
}

const words = (b: RubricBrief | undefined) => (b ? [b.film, b.visual, b.notes, ...(b.hardLines ?? [])].filter((x): x is string => typeof x === "string").join("\n") : "");

const TYPE_WORDS: ReadonlyArray<readonly [VideoType, RegExp]> = [
  ["meme", /\bmemes?\b|shitpost|funny edit|brainrot/i],
  ["gaming", /\bgaming\b|gameplay|game clip|twitch|esports|speedrun|\bfrag/i],
  ["music-video", /music video|lyric video|visuali[sz]er|official video/i],
  ["tutorial", /tutorial|\bhow to\b|step[- ]by[- ]step|\bguide\b|lesson|course|workshop/i],
  ["talk", /\btalk\b|conference|keynote|meetup|lecture|panel|podcast|presentation/i],
  ["vlog", /\bvlog|day in the life|behind the scenes/i],
  ["launch-promo", /launch|announc|release|teaser|promo\b|coming soon|is here|now available/i],
  ["product-demo", /\bdemo\b|walk ?-?through|feature tour|product tour/i],
  ["explainer", /explain|how it works|under the hood|deep dive/i],
  ["social", /cut-?downs?|tiktok|\breels?\b|\bshorts\b|social/i],
];

/**
 * The type a brief's own words point to (film, visual, notes), or undefined when they say nothing
 * clear (the agent passes one; a brief with none is judged as before D217).
 */
export function guessVideoType(b: RubricBrief): VideoType | undefined {
  const text = [b.film, b.visual, b.notes].filter(Boolean).join("\n");
  return TYPE_WORDS.find(([, re]) => re.test(text))?.[0];
}

/** Current skill identities, without repeats, in order. */
export const skillIdsOf = (ids: ReadonlyArray<string>) => [...new Set(ids)];

/**
 * The user asking for the music alone: "music only", "the song only", "only the music", "no (added)
 * sound effects", "no sfx" (T2's "The song only, no sound effects unless asked", A2r8's "Music only,
 * no added sound effects").
 */
const MUSIC_ONLY = /\b(music|song|track|soundtrack) only\b|\bonly (the |a |one )?(music|song|track|soundtrack)\b|\bno (added |extra |other )?(sound ?effects?|sfx|sound fx|foley|ui sounds?|click sounds?|clicks)\b|\bwithout (any )?(sound ?effects?|sfx)\b/i;

/**
 * The sound profile for a brief and the project's skills: the user's own words first (the request,
 * then the brief's), then the Sound design skill, then the type. Music only wins over every default:
 * nothing asks for effects the user ruled out (T2, A2r8: the request said music only, the profile
 * never read it, and the critics scored the silence down and asked for clicks). D306: and only the
 * user's request can say music only: the brief is the agent's plan, and an agent that wrote "music
 * only" there ruled out the keyboard and click sounds the user wanted.
 */
export function soundProfileFor(brief: RubricBrief | undefined, skills: ReadonlyArray<string> = [], request?: string): SoundProfile {
  skills = skillIdsOf(skills);
  const w = [request ?? "", words(brief)].filter(Boolean).join("\n");
  // "no sound effects" is music only, not silence: the music-only words first, and "no sound" only alone.
  if (request && MUSIC_ONLY.test(request) && !/\bno music\b|\bsilent\b|\bmuted\b/i.test(request)) return "music";
  if (/\bno (music|audio)\b|\bno sound\b(?! ?(effects?|fx|design))|\bsilent\b|\bmuted\b/i.test(w)) return "off";
  if (/\bsubtle sound|quiet sound|minimal sound|few sound effects|sound,? subtle/i.test(w)) return "strict";
  if (/\b(dense|loud|lots of|heavy) (sound|sfx|effects)|meme sounds|sound ?effects? everywhere/i.test(w)) return "loose";
  if (skills.includes("sound-design")) return "strict";
  return isVideoType(brief?.type) ? SOUND_BY_TYPE[brief.type] : "strict";
}

export interface Rubric {
  /** The brief's type, if it has one (none: judged as before D217). */
  readonly type?: VideoType;
  readonly levels: Readonly<Record<RubricCategory, RubricLevel>>;
  readonly sound: SoundProfile;
  /** Blocker kinds that fail a round; anything else the critic calls a blocker is a finding. */
  readonly hardKinds: ReadonlyArray<string>;
}

/**
 * The critic's rubric for a brief, the project's active skills and whether it has a brand kit
 * (D217): the type's table; no brand without a kit; the brief's words ("no hook", "no brand", "no
 * sound") take a category out; an active skill makes its category required (Launch promo the hook,
 * Motion craft the motion, Sound design the sound, Brand application the brand with a kit). Style
 * blockers fail a round only when the video asks for them: off-brand colour when brand is required,
 * sound when the sound profile is strict. Motion film (D292) makes the motion required too.
 * Without a type: everything required and every kind a blocker, as before.
 */
export function reviewRubric(brief: RubricBrief | undefined, o: { readonly skills?: ReadonlyArray<string>; readonly hasKit?: boolean; readonly request?: string } = {}): Rubric {
  const skills = skillIdsOf(o.skills ?? []);
  const sound = soundProfileFor(brief, skills, o.request);
  const type = isVideoType(brief?.type) ? brief.type : undefined;
  if (!type) {
    const levels = Object.fromEntries(RUBRIC_CATEGORIES.map((k) => [k, k === "sound" && sound === "off" ? N : R])) as Record<RubricCategory, RubricLevel>;
    return { levels, sound, hardKinds: [...DEFECT_KINDS, "brand-colour", "other", ...(sound === "strict" ? ["sound"] : [])] };
  }
  const row = TABLE[type];
  const levels = Object.fromEntries(RUBRIC_CATEGORIES.map((k, i) => [k, row[i]!])) as Record<RubricCategory, RubricLevel>;
  if (skills.includes("launch-promo")) levels.hook = R;
  if (skills.includes("motion-craft") || skills.includes("motion-film")) levels.motion = R;
  if (skills.includes("sound-design")) levels.sound = R;
  if (skills.includes("brand-application")) levels.brand = R;
  const w = words(brief);
  if (/\bno hook\b|hook (isn't|is not) needed|without a hook/i.test(w)) levels.hook = N;
  if (/\bno brand(ing)?\b|\bunbranded\b|without (a |the )?brand/i.test(w) || !o.hasKit) levels.brand = N;
  if (sound === "off") levels.sound = N;
  const hardKinds = [...DEFECT_KINDS, ...(levels.brand === R ? ["brand-colour"] : []), ...(sound === "strict" ? ["sound"] : [])];
  return { type, levels, sound, hardKinds };
}

/** A category's pass bar under a rubric (undefined: not scored). */
export const rubricBar = (level: RubricLevel) => (level === "required" ? REQUIRED_BAR : level === "optional" ? OPTIONAL_BAR : undefined);

/** The categories a rubric scores, in order. */
export const scoredCategories = (r: Pick<Rubric, "levels">) => RUBRIC_CATEGORIES.filter((k) => r.levels[k] !== "na");

// ---- findings the user marked Intended (D217) ----

/** A review finding as Intended sees it: its kind (a category or a blocker kind), a check's rule family, its time. */
export interface FindingRef {
  readonly kind?: string | undefined;
  readonly rule?: string | undefined;
  readonly at?: number | undefined;
  readonly to?: number | undefined;
}
export interface IntendedRef {
  readonly kind: string;
  readonly from?: number | undefined;
  readonly to?: number | undefined;
}
/** How far (s) a finding may move and still be the one the user marked. */
export const INTENDED_SLACK_S = 1;
/** What an Intended mark matches on: a check's rule family, else the finding's kind. */
export const findingKind = (f: FindingRef) => f.rule || f.kind || "defect";
/** The Intended entry for a finding (kind and time range; `what` and the time it was marked are the caller's). */
export const intendedOf = (f: FindingRef): IntendedRef => ({ kind: findingKind(f), ...(f.at !== undefined ? { from: f.at, to: Math.max(f.at, f.to ?? f.at) } : {}) });
/** Whether the user marked this finding Intended: the same kind, and its time within the marked range (give or take a second). */
export function isIntended(f: FindingRef, list: ReadonlyArray<IntendedRef> | undefined): boolean {
  if (!list?.length) return false;
  const kind = findingKind(f);
  return list.some((e) => {
    if (e.kind !== kind) return false;
    if (f.at === undefined || e.from === undefined) return f.at === undefined && e.from === undefined;
    const end = Math.max(f.at, f.to ?? f.at);
    return end >= e.from - INTENDED_SLACK_S && f.at <= (e.to ?? e.from) + INTENDED_SLACK_S;
  });
}
