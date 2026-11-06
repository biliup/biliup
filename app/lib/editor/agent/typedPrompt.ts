// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The typed prompt, honestly (A1-noskill4, A1-noskill): the request said "show me typing this
// prompt and the agent editing the video", and the film typed an invented one ("Cut a travel film
// from the drone shots, on the beat") while the critic judged only its readability. When the
// user's own message asks to show THIS prompt (my prompt, the prompt I typed) being typed, the
// words typed on screen are that message (shortened with an ellipsis when long, never replaced);
// when it doesn't, a made-up on-screen prompt is fine. Measured on the graphics' source (OCR isn't
// available, and the text census lists only small words): the composer's text, a literal or the
// const it names, against the message. English only and conservative, read clause by clause like
// asksOneTake: a miss leaves it to the critic's eye, a false ask would fail rounds.
import type { CriticFinding } from "./critic";
import { clauses, NEGATED } from "./motion";

/** The user's own words as an object of typing: "this prompt", "my request", "the prompt I typed", "what I wrote". */
const OWN = /\b(?:this|my|that|these|the same)\s+(?:exact\s+|very\s+|own\s+|actual\s+|same\s+)?(?:prompts?|requests?|messages?|words|brief|instructions?)\b|\bthe\s+(?:prompt|request|message|words|text)\s+(?:i|i've|i have)\s+(?:just\s+)?(?:typed|wrote|written|sent|gave|entered|pasted)\b|\bwhat\s+i\s+(?:just\s+)?(?:typed|wrote|sent|asked)\b/gi;
/** A typing word near it. */
const TYPING = /\b(?:typ(?:e|es|ed|ing)|enter(?:s|ed|ing)?|writ(?:e|es|ing)|written|wrote|past(?:e|es|ed|ing))\b/i;
/** How far (characters) the typing word may stand from the words it types. */
const NEAR = 40;

/**
 * Whether this message asks the film to show itself being typed: a clause where the user's own
 * words ("this prompt", "the prompt I typed") stand within NEAR characters of a typing word, not
 * negated, and not followed by a colon or a quote (a prompt given right there, "type this prompt:
 * 'Cut my clips'", is another prompt than the whole message). "a prompt", "any prompt" and "the
 * prompt" alone don't ask.
 */
export function asksTypedPrompt(text: string): boolean {
  for (const clause of clauses(text)) {
    for (const m of clause.matchAll(OWN)) {
      const at = m.index ?? 0;
      const end = at + m[0].length;
      if (NEGATED.test(clause.slice(0, at))) continue;
      if (/^\s*[:"'“‘«]/.test(clause.slice(end))) continue;
      const around = clause.slice(Math.max(0, at - NEAR), end + NEAR);
      if (TYPING.test(around)) return true;
    }
  }
  return false;
}

/** The message whose own words the film must type: the last of the user's messages that asks it, else none. */
export function typedPromptAsk(prompts: ReadonlyArray<string | undefined>): string | undefined {
  return prompts
    .map((x) => x?.trim() ?? "")
    .filter(Boolean)
    .filter(asksTypedPrompt)
    .at(-1);
}

const ELLIPSIS = /(?:…|\.{3})\s*$/;
/** Text as compared: one case, plain quotes and dashes, single spaces, no trailing punctuation or ellipsis. */
export const normText = (s: string) =>
  s
    .normalize("NFKC")
    .toLowerCase()
    .replace(/[‘’ʼ]/g, "'")
    .replace(/[“”]/g, '"')
    .replace(/[‐-―]/g, "-")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/[\s.!?,;:…]+$/, "");
/** Words a shortened prompt keeps at least (a shorter request is typed whole). */
export const MIN_KEPT_WORDS = 6;

/** Whether the typed text is the request: the same words, or its start cut with an ellipsis. */
export function typedMatches(typed: string, request: string): boolean {
  const t = normText(typed), r = normText(request);
  if (!t || !r) return false;
  if (t === r) return true;
  if (!ELLIPSIS.test(typed.trim())) return false;
  return r.startsWith(t) && t.split(" ").length >= Math.min(MIN_KEPT_WORDS, r.split(" ").length);
}

const LIT = String.raw`"(?:[^"\\\n]|\\.)*"|'(?:[^'\\\n]|\\.)*'|\x60(?:[^\x60\\$]|\\.|\$(?!\{))*\x60`;
const IDENT = String.raw`[A-Za-z_$][\w$]*`;
/** A literal's text as the page shows it: JS escapes (\\n, \\u2019, \\x27) and HTML entities and line-break tags read, so typography isn't a mismatch. */
const ENTITY: Readonly<Record<string, string>> = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: " ", rsquo: "\u2019", lsquo: "\u2018", rdquo: "\u201d", ldquo: "\u201c", hellip: "\u2026", mdash: "-", ndash: "-" };
const unquote = (lit: string) =>
  lit
    .slice(1, -1)
    .replace(/\\u\{([0-9a-f]+)\}|\\u([0-9a-f]{4})|\\x([0-9a-f]{2})/gi, (_, a, b, c) => String.fromCodePoint(parseInt(a ?? b ?? c, 16)))
    .replace(/\\([nrt])/g, " ")
    .replace(/\\(.)/g, "$1")
    .replace(/<br\s*\/?>/gi, " ")
    .replace(/&(?:#(\d+)|#x([0-9a-f]+)|([a-z]+));/gi, (m, d, h, n) => {if(!d&&!h)return ENTITY[n.toLowerCase()]??m;const code=parseInt(d??h,d?10:16);return code>=0&&code<=0x10ffff?String.fromCodePoint(code):m;});
/** A literal or literals joined with + ("…for Biliup, " + "this app…"): its text; undefined when a + joins something else. */
const LITS = String.raw`(?:${LIT})(?:\s*\+\s*(?:${LIT}))*(?!\s*\+)`;
const unquoteAll = (lits: string) => [...lits.matchAll(new RegExp(LIT, "g"))].map((m) => unquote(m[0])).join("");

/**
 * The text the graphics' composers show, as written in their source: an biliupUI composer piece's
 * or an editor's composer part's `text:` (a literal, or the const it names), else a string const
 * named like a prompt (PROMPT, typedPrompt) for a hand-built composer. `composer` says which kind
 * was found (only those are quoted when there are any); `named` keeps the prompt-named consts beside
 * composer texts (a hand-typed opening and a composer piece's follow-up), which may still match.
 */
export function composerTexts(sources: Iterable<string>): { readonly texts: ReadonlyArray<string>; readonly composer: boolean; readonly named?: ReadonlyArray<string> } {
  const fromComposer = new Set<string>();
  const named = new Set<string>();
  for (const src of sources) {
    const consts = new Map<string, string>();
    for (const m of src.matchAll(new RegExp(String.raw`\b(?:const|let|var)\s+(${IDENT})\s*=\s*(${LITS})`, "g"))) consts.set(m[1]!, unquoteAll(m[2]!));
    const value = (v: string) => (/^["'\x60]/.test(v) ? unquoteAll(v) : consts.get(v));
    const at = [
      new RegExp(String.raw`piece\s*:\s*["']composer["'][^}]*?\btext\s*:\s*(${LITS}|${IDENT})`, "g"),
      new RegExp(String.raw`\bcomposer\s*:\s*\{[^}]*?\btext\s*:\s*(${LITS}|${IDENT})`, "g"),
    ];
    for (const re of at)
      for (const m of src.matchAll(re)) {
        const v = value(m[1]!);
        if (v?.trim()) fromComposer.add(v.trim());
      }
    for (const [name, v] of consts) if (/prompt|typed/i.test(name) && v.trim().length >= 12 && /\s/.test(v.trim())) named.add(v.trim());
  }
  return fromComposer.size ? { texts: [...fromComposer], composer: true, ...(named.size ? { named: [...named] } : {}) } : { texts: [...named], composer: false };
}

/** The request as the fix quotes it: whole up to 160 characters, else its start to a word and an ellipsis. */
export function quotedAsk(request: string, cap = 160): string {
  const one = request.replace(/\s+/g, " ").trim();
  if (one.length <= cap) return one;
  const cut = one.slice(0, cap);
  return `${cut.slice(0, Math.max(cut.lastIndexOf(" "), cap / 2)).replace(/[\s,;:.]+$/, "")}…`;
}

const short = (s: string) => {
  const one = s.replace(/\s+/g, " ");
  return one.length > 90 ? `${one.slice(0, 89)}…` : one;
};

/**
 * The check for review_submit: a fact line for the critic and, when the composer types other words
 * than the message that asks for its own typing, a blocker (rule "typed-prompt", kind invented)
 * with the exact fix. Nothing when no message asks. With no composer text found in the source, the
 * fact says so and the critic reads the typed words on the frames.
 */
export function typedPromptCheck(prompts: ReadonlyArray<string | undefined>, sources: ReadonlyArray<string>): { readonly facts: ReadonlyArray<string>; readonly findings: ReadonlyArray<CriticFinding> } {
  const ask = typedPromptAsk(prompts);
  if (!ask) return { facts: [], findings: [] };
  const words = quotedAsk(ask);
  const fix = `type the user's own words in the composer: "${words}"${words.endsWith("…") ? " (shortened with an ellipsis, or all of it)" : ""}, the same words in the sent message, never another prompt`;
  const head = `The request asks to show its own prompt being typed, so the words typed in the film must be the user's message (cut with an ellipsis if long, never replaced)`;
  if (!sources.length) return { facts: [`${head}; Biliup found no graphic source to measure it: read the typed words on the frames and compare them with <request>.`], findings: [] };
  const { texts, composer, named = [] } = composerTexts(sources);
  if (!texts.length) {
    const r = normText(ask);
    const found = sources.some((s) => normText(s.replace(/\\(.)/g, "$1")).includes(r.split(" ").slice(0, MIN_KEPT_WORDS).join(" ")));
    return { facts: [`${head}; Biliup found no composer text in the graphics' source${found ? " (the request's words are in it)" : " and not the request's words either"}: read the typed words on the frames and compare them with <request>.`], findings: [] };
  }
  if (texts.some((t) => typedMatches(t, ask)) || named.some((t) => typedMatches(t, ask))) return { facts: [`${head}; measured in the graphics' source: the ${composer ? "composer" : "prompt"} text is the user's message.`], findings: [] };
  const typed = texts.slice(0, 2).map((t) => `"${short(t)}"`).join(" and ");
  return {
    facts: [],
    findings: [{ rule: "typed-prompt", key: "typed-prompt:source", hard: true, message: `The request asks to show its own prompt being typed, but the ${composer ? "composer" : "prompt"} in the graphics' source types ${typed}, not the user's words (measured in the source). Fix: ${fix}.` }],
  };
}
