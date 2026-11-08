// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Sound search in people's words (D208): meme names and their spellings ("fahh", "faaah", "vine
// boom", "ba dum tss"), synonyms ("swoosh" finds whooshes, "keyboard" finds typing), and a ranking
// by how well a sound matches, its licence and its length. Browser-safe: the Sound effects tab, the
// server's online search and the agent's library_search share it.
//
// Many meme sounds are clips from videos, games or TV, so nobody can give them away: no free source
// has the real "Fahh". An alias maps such a name to a search that finds free (CC0 / CC BY)
// sound-alikes and says so (`note`); the user's own folders hold the rest. No alias, name or variant
// names a brand or a person (issue #103, D278).

/** Fold Latin accents and case without discarding non-Latin file names or queries. */
export function normalizeText(text: string): string {
  return text
    .normalize("NFKD")
    .replace(/[̀-ͯ]/g, "")
    .normalize("NFC")
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\p{M}]+/gu, " ")
    .trim();
}

/** A word with its runs of a letter as one ("faaahhh" and "fah" are the same; "boom" is "bom"). */
export const squash = (word: string) => word.replace(/([a-z])\1+/g, "$1");

// Keep CJK phrases intact, but separate adjacent Latin names/numbers ("雨声Rain01").
// Do not collapse repeated CJK characters: "哈哈" and "哈" are different names.
const wordRuns = /[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Hangul}]+|[^\s\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Hangul}]+/gu;
const cjk = /[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Hangul}]/u;
export const wordsOf = (text: string) => (normalizeText(text).match(wordRuns) ?? []).map(squash);

/** The text as one squashed run with no spaces ("Vine-Boom", "vineboom" and "viiine boom" agree). */
const runOf = (text: string) => squash(normalizeText(text).replace(/ /g, ""));

/**
 * Words that mean the same sound, for search: a query word finds any of its group. Squashed (as
 * wordsOf gives them), so "boom" is written "bom" and "woosh" "wosh".
 */
const SYNONYM_GROUPS: ReadonlyArray<ReadonlyArray<string>> = [
  ["whosh", "swosh", "wosh", "swish", "swop", "whip", "swep"],
  ["swipe", "slide", "flick", "wipe"],
  ["click", "tap", "pres", "button", "mouse"],
  ["pop", "plop", "bubble", "blip", "drop"],
  ["bom", "impact", "hit", "thud", "bang", "punch", "thump"],
  ["riser", "rise", "build", "uplifter", "swel", "tension"],
  ["ding", "chime", "ping", "bel", "bong"],
  ["notification", "alert", "mesage"],
  ["suces", "win", "achievement", "complete", "done", "confirm", "level"],
  ["glitch", "distortion", "static", "interference", "stuter"],
  ["typing", "keyboard", "keys", "type", "keystroke"],
  ["aplause", "clap", "claping", "cheer"],
  ["laugh", "laughter", "laughing", "sitcom"],
  ["sparkle", "shimer", "twinkle", "magic", "shine"],
  ["eror", "wrong", "fail", "denied", "buzer"],
  ["camera", "shuter", "screnshot", "snap", "photo"],
  ["toggle", "switch"],
];
const SYNONYMS = new Map<string, ReadonlyArray<string>>();
for (const group of SYNONYM_GROUPS) for (const w of group) SYNONYMS.set(w, group);

// Search vocabulary, independent of interface language. Only these known terms are
// expanded for providers; arbitrary Chinese names and phrases go online unchanged.
const CHINESE_TERMS: ReadonlyArray<readonly [string, ...string[]]> = [
  ["whoosh", "呼啸", "呼嘯", "掠过", "掠過", "嗖", "唰"],
  ["transition", "转场", "轉場"],
  ["swipe", "滑动", "滑動", "划过", "劃過"],
  ["click", "点击", "點擊", "按键", "按鍵"],
  ["pop", "气泡", "氣泡", "啵"],
  ["impact", "冲击", "衝擊", "撞击", "撞擊", "重击", "重擊"],
  ["riser", "上升", "渐强", "漸強"],
  ["ding", "叮", "叮咚", "铃声", "鈴聲"],
  ["notification", "通知", "提示音"],
  ["success", "成功", "胜利", "勝利"],
  ["glitch", "故障", "干扰", "干擾", "失真"],
  ["typing", "打字", "键盘", "鍵盤"],
  ["applause", "鼓掌", "掌声", "掌聲", "欢呼", "歡呼"],
  ["laugh", "笑声", "笑聲", "大笑"],
  ["sparkle", "闪耀", "閃耀", "魔法"],
  ["error", "错误", "錯誤", "失败", "失敗"],
  ["camera", "快门", "快門", "拍照"],
  ["toggle", "开关", "開關"],
  ["rain", "雨", "雨声", "雨聲", "下雨"],
  ["wind", "风声", "風聲"],
  ["water", "流水", "水声", "水聲"],
  ["footsteps", "脚步", "腳步", "脚步声", "腳步聲"],
  ["ambient", "氛围", "氛圍", "环境", "環境"],
  ["cinematic", "电影感", "電影感"],
  ["upbeat", "欢快", "歡快"],
  ["piano", "钢琴", "鋼琴"],
  ["electronic", "电子", "電子"],
  ["guitar", "吉他"],
  ["gentle", "轻柔", "輕柔"],
  ["relaxing", "放松", "放鬆"],
];
const chineseOnline = new Map<string, string>();
for (const [online, ...names] of CHINESE_TERMS) {
  const word = squash(online);
  const group = [...(SYNONYMS.get(word) ?? [word]), ...names];
  for (const synonym of group) SYNONYMS.set(synonym, group);
  for (const name of names) chineseOnline.set(name, online);
}

/** A word and the words that mean the same. */
export const synonymsOf = (word: string): ReadonlyArray<string> => {
  const exact = squash(word);
  return [exact, ...(SYNONYMS.get(exact) ?? []).filter((term) => term !== exact)];
};

/** A sound people ask for by name: how they spell it, what to search for online, and the bundled sounds that answer it. */
export interface SoundAlias {
  readonly id: string;
  /** How the app writes it. */
  readonly name: string;
  /** What people type (spelling variants are covered by squash; these are the different words). */
  readonly variants: ReadonlyArray<string>;
  /** The online search that finds CC0 / CC BY sounds for it (each checked on Openverse, D208). */
  readonly online: string;
  /** A Freesound tag that narrows a Freesound search to it (with the user's key). */
  readonly tag?: string;
  /** Bundled sounds that answer it (ids in assets/sfx/library.json). */
  readonly bundled: ReadonlyArray<string>;
  /** The name is a clip nobody may give away: results are free sound-alikes. */
  readonly clip?: boolean;
  /** Shown as a meme chip in the Sound effects tab. */
  readonly chip?: boolean;
}

/**
 * Meme and popular sounds by name (D208). Every `online` query returned CC0 / CC BY results on
 * Openverse on 2026-10-14 ("fahh", "anime wow", "emotional damage" and "meme" return
 * none, so those names search for what the sound is instead).
 */
export const SOUND_ALIASES: ReadonlyArray<SoundAlias> = [
  { id: "fahh", name: "Fahh", variants: ["fahh", "faah", "fahhh", "fah meme"], online: "frustrated yell", tag: "yell", bundled: ["exaggerated-shout", "frustrated-yell"], clip: true, chip: true },
  { id: "boom", name: "Dramatic boom", variants: ["dramatic boom", "boom meme", "meme boom"], online: "bass boom", tag: "boom", bundled: ["dramatic-boom", "hit-deep-boom", "impact-boom"], chip: true },
  { id: "bruh", name: "Bruh", variants: ["bruh", "bro", "bruh moment"], online: "bruh", tag: "bruh", bundled: ["bruh"], chip: true },
  { id: "airhorn", name: "Airhorn", variants: ["airhorn", "air horn", "dj horn", "气喇叭", "氣喇叭"], online: "airhorn", tag: "airhorn", bundled: ["airhorn", "air-horn-blast"], chip: true },
  { id: "scratch", name: "Record scratch", variants: ["record scratch", "vinyl scratch", "scratch", "唱片刮擦", "搓碟"], online: "record scratch", tag: "scratch", bundled: ["record-scratch", "vinyl-scratch"], chip: true },
  { id: "trombone", name: "Sad trombone", variants: ["sad trombone", "wah wah", "womp womp", "wah wah wah", "fail trombone", "sad horn", "悲伤长号", "悲傷長號"], online: "sad trombone", tag: "trombone", bundled: ["sad-trombone", "womp-womp"], chip: true },
  { id: "wow", name: "Wow", variants: ["wow", "anime wow", "woah", "whoa"], online: "wow", tag: "wow", bundled: ["wow"], chip: true },
  { id: "oof", name: "Oof", variants: ["oof", "ouch"], online: "oof", tag: "oof", bundled: ["oof"], chip: true },
  { id: "crickets", name: "Crickets", variants: ["crickets", "cricket", "awkward silence", "awkward", "蟋蟀", "尴尬冷场", "尷尬冷場"], online: "crickets", tag: "crickets", bundled: ["crickets"], chip: true },
  { id: "rimshot", name: "Rimshot", variants: ["rimshot", "ba dum tss", "badum tss", "ba dum tiss", "badumtss", "joke drum"], online: "rimshot", tag: "rimshot", bundled: ["rimshot"], chip: true },
  { id: "drumroll", name: "Drum roll", variants: ["drum roll", "drumroll", "鼓滚奏", "鼓滾奏"], online: "drum roll", tag: "drumroll", bundled: ["drum-roll"], chip: true },
  { id: "boing", name: "Boing", variants: ["boing", "cartoon boing", "bounce", "spring", "弹簧", "彈簧", "卡通弹跳", "卡通彈跳"], online: "cartoon boing", tag: "boing", bundled: ["boing", "spring-boing"], chip: true },
  { id: "bassdrop", name: "Bass drop", variants: ["bass drop", "drop the bass", "wub", "低音下坠", "低音下墜"], online: "bass drop", tag: "bass-drop", bundled: ["bass-drop"], chip: true },
  { id: "ding", name: "Ding", variants: ["ding", "correct answer", "correct", "答对", "答對", "回答正确", "回答正確"], online: "ding", tag: "ding", bundled: ["ding", "bell-ding", "correct-ding", "glass-ping"], chip: true },
  { id: "laugh", name: "Laugh", variants: ["laugh track", "sitcom laugh", "crowd laugh", "laugh", "laughing"], online: "laugh", tag: "laugh", bundled: ["crowd-laugh"], chip: true },
  { id: "applause", name: "Applause", variants: ["applause", "clapping", "claps"], online: "applause", tag: "applause", bundled: ["applause"] },
  { id: "error", name: "Error", variants: ["error sound", "wrong buzzer", "wrong answer", "error"], online: "error", tag: "error", bundled: ["error", "wrong-buzzer"] },
  { id: "dramatic", name: "Dun dun dun", variants: ["dun dun dun", "dun dun", "dramatic sting"], online: "dun dun dun", tag: "dramatic", bundled: ["dun-dun-dun"], clip: true },
  { id: "bong", name: "Dramatic bong", variants: ["dramatic bong", "deep bong"], online: "bong", tag: "bong", bundled: ["deep-bong", "bong"] },
  { id: "huh", name: "Huh", variants: ["huh"], online: "huh", tag: "huh", bundled: ["questioning-huh"] },
  // D250: more names people search for, answered by Uncut's own synthesised sounds.
  { id: "bonk", name: "Bonk", variants: ["bonk", "bonk meme"], online: "bonk", tag: "bonk", bundled: ["bonk"] },
  { id: "kaching", name: "Cash register", variants: ["ka ching", "kaching", "cha ching", "cash register", "money sound", "收银机", "收銀機"], online: "cash register", tag: "cash-register", bundled: ["cash-register"] },
  { id: "slidewhistle", name: "Slide whistle", variants: ["slide whistle", "cartoon whistle"], online: "slide whistle", tag: "whistle", bundled: ["slide-whistle-up", "slide-whistle-down"] },
  { id: "levelup", name: "Level up", variants: ["level up", "levelup", "8 bit win", "chiptune win", "升级", "升級"], online: "level up", tag: "levelup", bundled: ["level-up", "coin"] },
  { id: "laser", name: "Laser", variants: ["laser", "pew pew", "pew", "zap", "激光", "镭射", "鐳射"], online: "laser", tag: "laser", bundled: ["laser-zap"] },
];

/** The alias a query names, when it names one (the whole query, spelling and spaces aside). */
export function aliasOf(query: string): SoundAlias | undefined {
  const run = runOf(query);
  if (!run) return undefined;
  return SOUND_ALIASES.find((a) => a.variants.some((v) => runOf(v) === run) || runOf(a.name) === run);
}

/** What a query asks for: its words, their synonyms, and the alias it names. */
export interface ExpandedQuery {
  readonly text: string;
  /** Each query word with the words that mean the same (all squashed). */
  readonly groups: ReadonlyArray<ReadonlyArray<string>>;
  readonly alias?: SoundAlias | undefined;
  /** What goes online: the alias's search or known Chinese terms; other text is preserved. */
  readonly online: string;
  /** Said with the results when the alias is a clip: why these are sound-alikes. */
  readonly note?: string | undefined;
}

export function clipNote(alias: SoundAlias) {
  return `"${alias.name}" is a clip from a video, so it isn't free to use: these are free sound-alikes. Add the clip from a folder of your own (Folders).`;
}

export function expandQuery(query: string): ExpandedQuery {
  const text = query.replace(/\s+/g, " ").trim();
  const alias = aliasOf(text);
  const groups = wordsOf(text).map((w) => synonymsOf(w));
  const online = alias?.online ?? text.replace(/\p{Script=Han}+/gu, (term) => {
    const known = chineseOnline.get(term);
    return known ? ` ${known} ` : term;
  }).replace(/\s+/g, " ").trim();
  return { text, groups, alias, online, ...(alias?.clip ? { note: clipNote(alias) } : {}) };
}

/** CJK names do not need spaces around a query; Latin words retain prefix matching. */
function hasWord(words: ReadonlySet<string>, term: string, prefix = false): boolean {
  return words.has(term) || [...words].some((word) => cjk.test(term)
    ? word.includes(term)
    : prefix && term.length >= 3 && word.startsWith(term));
}

function aliasInTitle(title: string, alias: SoundAlias): boolean {
  const run = runOf(title);
  return [alias.name, ...alias.variants, alias.online].map(runOf)
    .some((term) => (term.length > 2 || cjk.test(term)) && run.includes(term));
}

/** Something a search can rank: a title, tags, maybe a category name, a licence and a length. */
export interface Rankable {
  readonly title: string;
  readonly tags?: ReadonlyArray<string> | undefined;
  readonly category?: string | undefined;
  readonly license?: string | undefined;
  readonly seconds?: number | undefined;
}

/**
 * How well a sound answers a query (0: not at all): each query word (or a synonym) in the title
 * counts most, in a tag or the category less; an alias's variants and tags count too. A CC0 sound
 * and a length that suits the kind (a sound effect of 0.2 to 4 s; music over a minute) rank higher.
 */
export function scoreSound(sound: Rankable, q: ExpandedQuery, kind: "sfx" | "music" = "sfx"): number {
  const title = new Set(wordsOf(sound.title));
  const tags = new Set([...(sound.tags ?? []).flatMap(wordsOf), ...wordsOf(sound.category ?? "")]);
  let match = 0;
  let missing = 0;
  for (const group of q.groups) {
    const exact = group[0]!;
    if (hasWord(title, exact, true)) match += 3;
    else if (hasWord(tags, exact)) match += 2;
    else if (group.some((w) => hasWord(title, w))) match += 1.5;
    else if (group.some((w) => hasWord(tags, w))) match += 1;
    else missing++;
  }
  if (q.alias) {
    if (aliasInTitle(sound.title, q.alias)) match += 4;
    else if (wordsOf(q.alias.online).every((w) => title.has(w) || tags.has(w))) match += 2;
    if (q.alias.tag && tags.has(squash(q.alias.tag.replace(/-/g, " ").split(" ")[0]!))) match += 1;
  }
  if (match === 0) return 0;
  if (q.groups.length && missing === q.groups.length && !q.alias) return 0;
  let score = match - missing * 2;
  if (sound.license === "cc0" || sound.license === "CC0-1.0") score += 1.5;
  const s = sound.seconds;
  if (s !== undefined) {
    if (kind === "sfx") score += s >= 0.15 && s <= 4 ? 2 : s <= 10 ? 1 : s > 30 ? -2 : 0;
    else score += s >= 60 ? 1 : s < 15 ? -1 : 0;
  }
  return Math.max(score, 0.01);
}

/** The items best first (stable: equal scores keep the source's order); those that don't match at all stay, last. */
export function rankSounds<T extends Rankable>(items: ReadonlyArray<T>, query: string, kind: "sfx" | "music" = "sfx"): Array<T> {
  const q = expandQuery(query);
  return items
    .map((item, i) => ({ item, i, score: scoreSound(item, q, kind) }))
    .sort((a, b) => b.score - a.score || a.i - b.i)
    .map((e) => e.item);
}

/**
 * Whether a sound matches a query for a local list (the bundled pack, the user's folders): every
 * query word (or a synonym, or a word it starts) is in its title, tags, category or extra words,
 * or the query names an alias the sound answers.
 */
export function matchesQuery(sound: Rankable & { readonly id?: string; readonly extra?: string }, q: ExpandedQuery): boolean {
  if (!q.groups.length) return true;
  if (q.alias && ((sound.id && q.alias.bundled.includes(sound.id)) || aliasInTitle(sound.title, q.alias))) return true;
  const hay = new Set([...wordsOf(sound.title), ...(sound.tags ?? []).flatMap(wordsOf), ...wordsOf(sound.category ?? ""), ...wordsOf(sound.extra ?? "")]);
  return q.groups.every((group) => group.some((w) => hasWord(hay, w, true)));
}
