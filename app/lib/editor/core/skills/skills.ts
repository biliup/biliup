// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Built-in editing recipes and checklist parsing shared by the editor and assistant.
import { isVideoType, type VideoType } from "@/app/lib/editor/core/skills/rubric";

/** Built-in editing skills, addressed by the same names in the panel and chat. */
export const BUILTIN_IDS = {
  launch: "launch-promo",
  demo: "product-demo",
  talking: "talking-head",
  social: "social-cutdown",
  explainer: "explainer",
  titles: "title-sequence",
  motion: "motion-craft",
  sound: "sound-design",
  brand: "brand-application",
  /** D232: the skill "Make from reference" in the Brand tab calls. */
  brandKit: "make-brand-kit",
  /** D292: a one-take, never-cut motion film on the beat (render(t) graphics, biliup:lib/motion.js). */
  film: "motion-film",
} as const;


/** A checklist's heading: "Review checklist", "Checklist", "Check before handing it over". */
const CHECK_HEAD = /^\s*(review checklist|checklist|check before handing it over|before handing it over)\b/i;
const BULLET = /^\s*[-–]\s+(.*\S)\s*$/;

/** The "Review checklist" lines of a skill's body (the critic checks them), at most 12. */
export function skillChecklist(body: string): Array<string> {
  const lines = body.split(/\r?\n/);
  const at = lines.findIndex((l) => CHECK_HEAD.test(l));
  if (at < 0) return [];
  const out: Array<string> = [];
  for (const l of lines.slice(at + 1)) {
    const m = BULLET.exec(l);
    if (m) out.push(m[1]!.slice(0, 240));
    else if (l.trim() && out.length) break;
  }
  return out.slice(0, 12);
}

/** What suggestSkills reads of a brief. */
export interface BriefLike {
  readonly film: string;
  readonly audience?: string;
  readonly duration?: string;
  readonly formats?: ReadonlyArray<string>;
  readonly assets?: ReadonlyArray<string>;
  readonly visual?: string;
  readonly deliverables?: ReadonlyArray<string>;
  readonly notes?: string;
}

const has = (re: RegExp, ...xs: ReadonlyArray<string | ReadonlyArray<string> | undefined>) => xs.some((x) => (Array.isArray(x) ? x.some((y) => re.test(y)) : typeof x === "string" && re.test(x)));
/** English/Chinese time units, compound durations and clocks; a range uses its longer end. */
export function durationSeconds(d: string | undefined): number | undefined {
  if (!d || /^\s*[-−]/.test(d)) return undefined;
  const parse = (text: string): number | undefined => {
    if (/^\s*[-−]/.test(text)) return undefined;
    const clock = /(?:^|[^\d:])(\d+):(\d{2})(?::(\d{2}))?(\.\d+)?(?![\d:])/.exec(text);
    if (clock) {
      const middle = Number(clock[2]), last = clock[3] === undefined ? undefined : Number(clock[3]);
      if (middle >= 60 || (last !== undefined && last >= 60)) return undefined;
      return (last === undefined ? Number(clock[1]) * 60 + middle : Number(clock[1]) * 3600 + middle * 60 + last) + Number(clock[4] ?? 0);
    }
    const units = [...text.matchAll(/(\d+(?:\.\d+)?)\s*(毫秒|milliseconds?|ms|小时|hours?|hrs?|h|分钟|分|minutes?|mins?|m|秒钟|秒|seconds?|secs?|s)(?![a-z])/gi)];
    if (units.length) return units.reduce((sum, match) => {
      const unit = match[2]!.toLowerCase();
      const scale = /^(?:毫秒|milliseconds?|ms)$/.test(unit) ? 0.001 : /^(?:小时|hours?|hrs?|h)$/.test(unit) ? 3600 : /^(?:分钟|分|minutes?|mins?|m)$/.test(unit) ? 60 : 1;
      return sum + Number(match[1]) * scale;
    }, 0);
    return /^\s*\d+(?:\.\d+)?\s*$/.test(text) ? Number(text) : undefined;
  };
  const parts = d.trim().replace(/(\d)\s*-\s*(?=[a-z]|小时|分钟|分|秒|毫秒)/gi, '$1 ').split(/\s*(?:[-–—~～]|到|至|\bto\b)\s*/i);
  const values = parts.map(parse).filter((value): value is number => value !== undefined && Number.isFinite(value) && value > 0);
  return values.length ? Math.max(...values) : undefined;
}
const ratio = (f: string) => {
  const m = /(\d+)\s*[:x×]\s*(\d+)/.exec(f);
  if (!m) return undefined;
  const a = Number(m[1]);
  const b = Number(m[2]);
  // "1080x1920" or "9:16"
  return a > 0 && b > 0 ? a / b : undefined;
};

const TYPE_SKILL: Readonly<Record<VideoType, string | undefined>> = {
  "launch-promo": BUILTIN_IDS.launch,
  "product-demo": BUILTIN_IDS.demo,
  tutorial: BUILTIN_IDS.demo,
  talk: BUILTIN_IDS.talking,
  vlog: BUILTIN_IDS.talking,
  social: BUILTIN_IDS.social,
  explainer: BUILTIN_IDS.explainer,
  "music-video": undefined,
  meme: BUILTIN_IDS.social,
  gaming: BUILTIN_IDS.social,
  other: undefined,
};
/** Types whose videos usually have motion graphics (Motion craft is suggested). */
const MOTION_TYPES: ReadonlySet<string> = new Set(["launch-promo", "explainer", "social", "music-video"]);
/** Types whose sound design is part of the job (Sound design is suggested). */
const SOUND_TYPES: ReadonlySet<string> = new Set(["launch-promo", "product-demo"]);
/** Types that carry a brand (Brand application is suggested with a kit). */
const BRAND_TYPES: ReadonlySet<string> = new Set(["launch-promo", "product-demo", "tutorial", "talk", "social", "explainer", "other"]);

/**
 * The built-in skills that fit a brief (D207, D217): suggestions only, never turned on by
 * themselves; the user picks (on an ask_user card or the Brief card). The type's own kind of film,
 * then one or two more from the brief's words, formats and length (talking head, launch, demo,
 * explainer over 45 s, titles, social when it has several aspect ratios or asks for cut-downs; a
 * brief with no type and nothing found: Launch promo); Motion craft only for videos with motion
 * graphics (a promo, an explainer, a social cut, a music video, titles), Sound design for a promo or
 * a product video, Brand application with a kit for a branded type; Motion film (D292) after the kinds
 * when the brief asks for one take, no cuts, a morph or a seamless loop (D297: then not Motion craft,
 * whose one-thing-at-a-time rules contradict it). In the order they're listed in.
 */
export function suggestSkills(brief: BriefLike & { readonly type?: string }, o: { readonly hasKit?: boolean } = {}): Array<string> {
  const type = isVideoType(brief.type) ? brief.type : undefined;
  const words = [brief.film, brief.visual, brief.notes, brief.audience];
  const picked = new Set<string>();
  const own = type ? TYPE_SKILL[type] : undefined;
  if (own) picked.add(own);
  const talking = has(/talking|to camera|interview|founder|speaker|podcast|webcam|selfie|vlog|presenter|keynote clip/i, ...words, brief.assets);
  const demo = has(/\bdemo|walk ?-?through|tutorial|how to use|screen ?(record|cap)|recording|capture|\bui\b|app flow|onboarding|feature tour/i, ...words, brief.assets);
  const explainer = has(/explain|how it works|under the hood|architecture|concept|diagram|why it|deep dive/i, ...words);
  const titles = has(/title sequence|titles|kinetic|typograph|lyric|type-?only|text-?only|manifesto|opener|intro sequence/i, ...words);
  const oneTake = has(/one[- ]take|never cut|no cuts|one continuous (shot|take)|motion film|motion study|one shape|morph(s|ing)?\b|keynote[- ]style|seamless loop/i, ...words);
  const launch = has(/launch|announc|release|teaser|promo|introduc|coming soon|is here|now available|\bship|\bv\d|version \d/i, ...words);
  const formats = (brief.formats ?? []).map(ratio).filter((x): x is number => x !== undefined);
  const kinds = new Set(formats.map((r) => (r < 0.7 ? "tall" : r < 0.9 ? "4:5" : r < 1.1 ? "square" : "wide")));
  const multi = kinds.size > 1 || has(/cut-?downs?|every format|all formats|multi-?format|reels.*shorts|shorts.*tiktok/i, ...words, brief.deliverables);
  const length = durationSeconds(brief.duration);
  if (talking) picked.add(BUILTIN_IDS.talking);
  if (launch || (!type && !talking && !demo && !explainer && !titles)) picked.add(BUILTIN_IDS.launch);
  if (demo) picked.add(BUILTIN_IDS.demo);
  if (explainer || (length !== undefined && length > 45 && !talking)) picked.add(BUILTIN_IDS.explainer);
  if (titles) picked.add(BUILTIN_IDS.titles);
  if (multi) picked.add(BUILTIN_IDS.social);
  // At most two kinds of film, the first ones found.
  const kindsPicked = [...picked].slice(0, 2);
  const motion = type ? MOTION_TYPES.has(type) || kindsPicked.some((k) => k === BUILTIN_IDS.launch || k === BUILTIN_IDS.titles) : true;
  const sound = type ? SOUND_TYPES.has(type) || kindsPicked.includes(BUILTIN_IDS.launch) : true;
  const brand = !!o.hasKit && (!type || BRAND_TYPES.has(type));
  const out = [...kindsPicked, ...(oneTake ? [BUILTIN_IDS.film] : []), ...(motion && !oneTake ? [BUILTIN_IDS.motion] : []), ...(sound ? [BUILTIN_IDS.sound] : []), ...(brand ? [BUILTIN_IDS.brand] : [])];
  if (multi && !out.includes(BUILTIN_IDS.social)) out.splice(kindsPicked.length, 0, BUILTIN_IDS.social);
  return out;
}

export { findingKind, guessVideoType, skillIdsOf, intendedOf, isIntended, isVideoType, OPTIONAL_BAR, REQUIRED_BAR, reviewRubric, RUBRIC_CATEGORIES, rubricBar, scoredCategories, soundProfileFor, VIDEO_TYPE_NAMES, VIDEO_TYPES, DEFECT_KINDS } from "@/app/lib/editor/core/skills/rubric";
export type { FindingRef, IntendedRef, Rubric, RubricBrief, RubricCategory, RubricLevel, VideoType } from "@/app/lib/editor/core/skills/rubric";
