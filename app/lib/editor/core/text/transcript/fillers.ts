// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type SpeechSegment, type SpeechEnvelope, speechWords, silenceIn, ms3 } from "@/app/lib/editor/core/text/transcript/speech";

/** A gap between words at least this long (seconds) is a pause. */
export const PAUSE_MIN_S = 0.3;

// Conservative on purpose: hesitation sounds are safe to cut; discourse words ("no", "znaczy",
// "you know") often carry meaning, so they're marked "maybe" for the agent to judge in context.
// A list applies only to its language ("no" is a Polish filler and an English word).
const HESITATIONS: Readonly<Record<string, ReadonlyArray<string>>> = {
  en: ["um", "umm", "uh", "uhh", "uhm", "er", "erm", "hmm", "mm", "mmm"],
  pl: ["yyy", "yy", "yyyy", "eee", "ee", "eeee", "eem", "yyym", "hmm", "mmm", "mhm"],
  zh: ["呃", "呃呃"],
};
const DISCOURSE: Readonly<Record<string, ReadonlyArray<string>>> = {
  en: ["you know", "i mean"],
  pl: ["no", "znaczy", "jakby", "tak jakby", "wiesz"],
  // These can carry meaning (especially an affirmative 嗯); only mark them for review.
  zh: ["嗯", "啊", "额", "額", "然后", "然後", "就是", "那个", "那個", "这个", "這個", "怎么说", "怎麼說", "你知道", "就是说", "就是說"],
};
export const FILLERS = { hesitation: HESITATIONS, discourse: DISCOURSE } as const;

const bare = (text: string) => text.toLowerCase().replace(/[^\p{L}\p{N}' ]+/gu, "").trim();
const clauseEnd = /[,.?!…，。！？、；：]["'”’)\]）】》〉」』]*$/u;

/** Match whole timed tokens. Never split one token and invent times for its characters. */
function phraseSize(words:ReadonlyArray<{readonly text:string}>,start:number,phrase:string,compact:boolean):number {
  if(!compact){
    const parts=phrase.split(' ');
    return parts.every((part,k)=>start+k<words.length&&bare(words[start+k]!.text)===part)?parts.length:0;
  }
  let text='';
  for(let n=0;start+n<words.length&&n<phrase.length;n++){
    text+=bare(words[start+n]!.text).replace(/\s/g,'');
    if(text===phrase)return n+1;
    if(!phrase.startsWith(text)||clauseEnd.test(words[start+n]!.text))break;
  }
  return 0;
}

export interface Filler {
  /** Index of its first word, and how many words. */
  readonly i: number;
  readonly n: number;
  readonly kind: "hesitation" | "maybe";
}

/** The fillers among `words` for `language` (other languages: none; whisper rarely writes them anyway). */
export function fillersOf(words: ReadonlyArray<{ readonly text: string }>, language: string | undefined): Array<Filler> {
  const lang = language?.toLowerCase().split(/[-_]/)[0];
  if (!lang||!Object.hasOwn(HESITATIONS,lang)) return [];
  const hes = new Set(HESITATIONS[lang]);
  const phrases = [...DISCOURSE[lang]!].sort((a, b) => b.length - a.length);
  const out: Array<Filler> = [];
  for (let i = 0; i < words.length; i++) {
    const w = bare(words[i]!.text);
    if (hes.has(w)) {
      // Without punctuation an isolated 呃 token may be part of a word such as 呃逆.
      out.push({ i, n: 1, kind: lang==='zh'&&!clauseEnd.test(words[i]!.text)?'maybe':"hesitation" });
      continue;
    }
    for (const phrase of phrases) {
      const n=phraseSize(words,i,phrase,lang==='zh');
      if (n) {
        // A discourse word only counts when it stands apart: "no" ending a clause or followed by a comma.
        const lastText = words[i + n - 1]!.text;
        const prevText = i > 0 ? words[i - 1]!.text : ".";
        if ((lang!=='zh'&&n>1) || clauseEnd.test(lastText) || clauseEnd.test(prevText)) {
          out.push({ i, n, kind: "maybe" });
          i += n - 1;
        }
        break;
      }
    }
  }
  return out;
}

export interface Pause {
  /** Seconds of the file. */
  readonly start: number;
  readonly end: number;
  /** The word before it (index), or -1 before the first word. */
  readonly after: number;
  readonly edge?: "start" | "end";
  /** With the loudness analysis cached: whether the sound is below its silence threshold here (start/end then come from the silence). */
  readonly silent?: boolean;
}

/**
 * Gaps of at least `min` seconds between words, and before the first and after the last, between
 * where the words are heard (speechWords: whisper's words, its segments' edges and the silences).
 * With `silences` (loudness analyzer, seconds), a pause overlapping one starts where the word
 * before it ends or the silence starts, whichever is later (a −40 dB silence can start in a word's
 * quiet decay), and ends where the silence ends (onsets are sharp; it may take the start of a word
 * whisper timed too early, never past its end), and says silent; one overlapping none says
 * silent: false (breath, noise or music there).
 */
export function pausesOf(words: ReadonlyArray<{ readonly start: number; readonly end: number }>, seconds: number | undefined, silences?: ReadonlyArray<{ readonly start: number; readonly end: number }>, min = PAUSE_MIN_S, segments?: ReadonlyArray<SpeechSegment>, envelope?: SpeechEnvelope): Array<Pause> {
  const out: Array<Pause> = [];
  if (!words.length) return out;
  const heard = speechWords(words, { segments, silences, seconds, envelope });
  const gaps: Array<{ start: number; end: number; after: number; edge?: "start" | "end"; hi: number }> = [];
  gaps.push({ start: 0, end: heard[0]!.start, after: -1, edge: "start", hi: heard[0]!.end });
  for (let i = 1; i < heard.length; i++) gaps.push({ start: heard[i - 1]!.end, end: heard[i]!.start, after: i - 1, hi: heard[i]!.end });
  const last = heard.at(-1)!;
  if (seconds !== undefined) gaps.push({ start: last.end, end: Math.max(seconds, last.end), after: heard.length - 1, edge: "end", hi: seconds });
  for (const g of gaps) {
    let { start, end } = g;
    let silent: boolean | undefined;
    if (silences) {
      const best = silenceIn(silences, start, end);
      silent = best !== undefined;
      if (best) {
        start = Math.max(best.start, g.start);
        end = Math.max(start, Math.min(best.end, g.hi));
      }
    }
    if (end - start < min - 1e-9) continue;
    out.push({ start: ms3(start), end: ms3(end), after: g.after, ...(g.edge ? { edge: g.edge } : {}), ...(silent !== undefined ? { silent } : {}) });
  }
  return out;
}

// ---- cutting by text (D69) ----

/** Seconds of sound kept before a word that stays, at a cut (onsets are sharp; ~67 ms). */
export const HEAD_S = 0.06;
/** Seconds kept after a word that stays (its decay and final consonant fade slowly, D74). */
export const TAIL_S = 0.15;

/**
 * Frames kept before a word that stays, on each cut next to it: 2 at 24–30 fps, 3 at 50, 4 at 60
 * (HEAD_S, at least 2). A removed pause keeps this much before the word after it too.
 */
export const handleFrames = (fps: number) => Math.max(2, Math.ceil(HEAD_S * fps - 1e-6));

/**
 * Frames kept after a word that stays (D74): TAIL_S, at least 3: 5 at 30 fps, 4 at 24–25, 8 at 50,
 * 9 at 60. Longer than the head: a word's end fades out slowly (below a −40 dB silence's start)
 * and whisper's ends run early, so a cut right after one clips its tail.
 */
export const tailFrames = (fps: number) => Math.max(3, Math.ceil(TAIL_S * fps - 1e-6));

/** Removing pauses: those at least this long (seconds) by default (the panel's threshold). */
export const DEFAULT_PAUSE_MIN_S = 0.5;
