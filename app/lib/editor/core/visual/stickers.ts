// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Stickers (F22, D165; the bigger set in D251): a bundled set the Elements tab searches, no network.
// Emoji are drawn by the system's colour emoji font (a name and keywords each, our own list); the
// drawn stickers are SVG we made for Uncut (own work, CC0), kept here so the preview and the export
// draw the same markup. A sticker item is a component.sticker with props.emoji or props.art (an id
// below).
//
// A drawn sticker is either a 200 × 200 picture (badges, bubbles, bursts, social glyphs), drawn
// square in its box, or a mark (a hand-drawn circle, an underline, a highlighter stroke), drawn by a
// function of its box's aspect so it stretches to whatever it circles or underlines without
// squashing its strokes. Placeholders in the markup: {{c}} the main colour (the sticker's `color`
// prop, else the art's own), {{d}} the dash offset of strokes that draw on (1 → 0), {{f}} the
// opacity of the parts that appear once the stroke is drawn (an arrow's head), {{t}} / {{ts}} the
// words and their size on the ones that carry words (a badge's "SALE", a bubble's "Hi!").
import { esc, num } from "@/app/lib/editor/core/visual/render";

export interface StickerEntry {
  /** "emoji:<character>" or "art:burst". */
  readonly key: string;
  readonly name: string;
  readonly keywords: ReadonlyArray<string>;
  /** Its section in the Elements tab (STICKER_GROUPS). */
  readonly group: StickerGroup;
  readonly emoji?: string;
  readonly art?: string;
}

/** The sticker sections, in the Elements tab's order. */
export const STICKER_GROUPS = ["Reactions", "Arrows & marks", "Shapes & bursts", "Bubbles", "Badges", "Social", "Emoji"] as const;
export type StickerGroup = (typeof STICKER_GROUPS)[number];

/** [emoji, name, keywords]: a small everyday set for video overlays. Through "hundred": Reactions; the rest: Emoji. */
const EMOJI: ReadonlyArray<readonly [string, string, string]> = [
  ["🔥", "fire", "hot lit flame trending"],
  ["😂", "laughing", "lol funny joy tears haha"],
  ["🤣", "rolling laughing", "lol funny rofl haha"],
  ["😍", "heart eyes", "love crush adore"],
  ["🥰", "smiling hearts", "love cute adore"],
  ["😎", "cool", "sunglasses chill smooth"],
  ["🤔", "thinking", "hmm wonder question"],
  ["🤯", "mind blown", "wow shocked explode amazing"],
  ["😱", "screaming", "shock scared omg wow"],
  ["😮", "surprised", "wow omg open mouth"],
  ["😭", "crying", "sad tears sob"],
  ["😢", "sad", "tear cry unhappy"],
  ["😅", "sweat smile", "phew nervous relief"],
  ["😊", "smile", "happy blush glad"],
  ["😀", "grin", "happy smile face"],
  ["😉", "wink", "flirt joke"],
  ["🙃", "upside down", "silly irony"],
  ["😴", "sleeping", "tired zzz bored"],
  ["🥳", "party face", "celebrate birthday hooray"],
  ["🤩", "star struck", "wow amazing excited"],
  ["😡", "angry", "mad rage furious"],
  ["🤬", "cursing", "angry swear mad"],
  ["🤫", "shush", "quiet secret silence"],
  ["🫡", "salute", "respect yes sir"],
  ["🥲", "smile tear", "bittersweet touched"],
  ["😬", "grimace", "awkward oops yikes"],
  ["🙄", "eye roll", "whatever annoyed bored"],
  ["🤡", "clown", "silly joke fool"],
  ["💀", "skull", "dead lol dying"],
  ["👻", "ghost", "boo spooky halloween"],
  ["👽", "alien", "ufo space"],
  ["🤖", "robot", "ai bot machine tech"],
  ["💩", "poop", "funny bad"],
  ["👍", "thumbs up", "like yes ok good approve"],
  ["👎", "thumbs down", "dislike no bad"],
  ["👏", "clap", "applause bravo congrats"],
  ["🙌", "raised hands", "hooray celebrate praise"],
  ["🙏", "pray", "please thanks hope"],
  ["💪", "muscle", "strong power gym flex"],
  ["👀", "eyes", "look watch see"],
  ["👋", "wave", "hello hi bye"],
  ["👉", "point right", "this here next"],
  ["👈", "point left", "this here back"],
  ["👆", "point up", "above this"],
  ["👇", "point down", "below this subscribe"],
  ["✌️", "peace", "victory two"],
  ["🤞", "fingers crossed", "luck hope"],
  ["🤝", "handshake", "deal agree partner"],
  ["✍️", "writing", "note write"],
  ["🧠", "brain", "smart think idea"],
  ["❤️", "red heart", "love like"],
  ["💔", "broken heart", "sad breakup"],
  ["💖", "sparkling heart", "love cute"],
  ["💯", "hundred", "perfect score 100 keep it real"],
  ["✨", "sparkles", "magic shine new clean"],
  ["⭐", "star", "favourite favorite rating"],
  ["🌟", "glowing star", "shine special"],
  ["⚡", "lightning", "fast power energy zap"],
  ["💥", "boom", "explosion bang collision"],
  ["💫", "dizzy", "stars magic"],
  ["💬", "speech bubble", "chat talk comment"],
  ["💭", "thought bubble", "think idea"],
  ["💡", "light bulb", "idea tip smart"],
  ["❓", "question", "what ask help"],
  ["❗", "exclamation", "important alert warning"],
  ["⚠️", "warning", "caution alert danger"],
  ["✅", "check", "done yes correct tick ok"],
  ["❌", "cross", "no wrong cancel x"],
  ["🚫", "prohibited", "no forbidden stop"],
  ["🛑", "stop", "halt sign"],
  ["🎉", "party popper", "celebrate congrats tada hooray"],
  ["🎊", "confetti", "celebrate party"],
  ["🎁", "gift", "present giveaway"],
  ["🏆", "trophy", "win winner champion award"],
  ["🥇", "gold medal", "first winner"],
  ["🎯", "target", "goal bullseye aim"],
  ["🚀", "rocket", "launch fast ship space"],
  ["📈", "chart up", "growth increase stonks"],
  ["📉", "chart down", "decrease loss"],
  ["💰", "money bag", "cash rich dollar"],
  ["💸", "money flying", "spend cash"],
  ["💎", "gem", "diamond value"],
  ["⏰", "alarm clock", "time wake deadline"],
  ["⏳", "hourglass", "time wait loading"],
  ["📌", "pin", "pinned important"],
  ["📍", "location", "place map here"],
  ["🔔", "bell", "notification subscribe alert"],
  ["🔒", "lock", "secure private"],
  ["🔑", "key", "access unlock"],
  ["🛠️", "tools", "build fix repair"],
  ["⚙️", "gear", "settings config"],
  ["💻", "laptop", "computer code work"],
  ["📱", "phone", "mobile app"],
  ["⌨️", "keyboard", "type code"],
  ["🎥", "movie camera", "film video record"],
  ["📸", "camera flash", "photo picture"],
  ["🎬", "clapper", "action film scene cut"],
  ["🎤", "microphone", "sing talk podcast"],
  ["🎧", "headphones", "music listen audio"],
  ["🎵", "music note", "song melody"],
  ["🎮", "game", "controller play gaming"],
  ["📚", "books", "learn study read"],
  ["📝", "memo", "notes write list"],
  ["🔍", "magnifier", "search find zoom look"],
  ["🔗", "link", "chain url"],
  ["📦", "package", "box ship delivery"],
  ["🐛", "bug", "error issue insect"],
  ["☕", "coffee", "drink morning break"],
  ["🍕", "pizza", "food lunch"],
  ["🍔", "burger", "food lunch"],
  ["🍿", "popcorn", "movie watch snack"],
  ["🍻", "cheers", "beer drinks toast"],
  ["🎂", "cake", "birthday celebrate"],
  ["🌈", "rainbow", "pride colours colors"],
  ["☀️", "sun", "sunny summer weather"],
  ["🌙", "moon", "night sleep"],
  ["❄️", "snowflake", "cold winter"],
  ["🌊", "wave water", "ocean sea surf"],
  ["🌍", "earth", "world globe planet"],
  ["🌸", "blossom", "flower spring"],
  ["🌵", "cactus", "desert plant"],
  ["🐶", "dog", "puppy pet"],
  ["🐱", "cat", "kitten pet"],
  ["🦄", "unicorn", "magic rare"],
  ["🐐", "goat", "greatest best"],
  ["🦋", "butterfly", "change pretty"],
  ["✈️", "plane", "travel flight trip"],
  ["🚗", "car", "drive road"],
  ["🏁", "finish flag", "race end done"],
  ["➡️", "arrow right", "next forward"],
  ["⬅️", "arrow left", "back previous"],
  ["⬆️", "arrow up", "up above"],
  ["⬇️", "arrow down", "down below"],
  ["🔄", "repeat", "refresh loop again"],
  ["🆕", "new", "fresh latest"],
  ["🆒", "cool button", "cool"],
  ["🔴", "red circle", "record live dot"],
  ["🟢", "green circle", "go online dot"],
];

// ---- drawing helpers (all coordinates in the art's view box) ----

const INK = "#111111";
/** A stroke that draws on: pathLength 1, dashed, offset {{d}}. */
const DRAW = `pathLength="1" stroke-dasharray="1 1" stroke-dashoffset="{{d}}"`;
/** The words of a sticker that carries them, centred on (x, y). */
const words = (x: number, y: number, fill: string, extra = "") => `<text x="${x}" y="${y}" text-anchor="middle" dominant-baseline="central" font-family="'DM Sans',sans-serif" font-weight="900" font-size="{{ts}}" fill="${fill}"${extra}>{{t}}</text>`;
/** A smooth path through points (Catmull-Rom as cubic Béziers). */
function smooth(points: ReadonlyArray<readonly [number, number]>): string {
  const p = points;
  let d = `M${num(p[0]![0])} ${num(p[0]![1])}`;
  for (let i = 0; i < p.length - 1; i++) {
    const p0 = p[i - 1] ?? p[i]!;
    const p1 = p[i]!;
    const p2 = p[i + 1]!;
    const p3 = p[i + 2] ?? p2;
    d += `C${num(p1[0] + (p2[0] - p0[0]) / 6)} ${num(p1[1] + (p2[1] - p0[1]) / 6)} ${num(p2[0] - (p3[0] - p1[0]) / 6)} ${num(p2[1] - (p3[1] - p1[1]) / 6)} ${num(p2[0])} ${num(p2[1])}`;
  }
  return d;
}
/** A star or seal outline: `n` points alternating between radius R and r. */
function spikes(n: number, R: number, r: number, cx = 100, cy = 100, turn = -90): string {
  const pts: Array<string> = [];
  for (let i = 0; i < n * 2; i++) {
    const a = ((turn + (i * 180) / n) * Math.PI) / 180;
    const rad = i % 2 ? r : R;
    pts.push(`${num(cx + rad * Math.cos(a))},${num(cy + rad * Math.sin(a))}`);
  }
  return pts.join(" ");
}
/** A four-pointed sparkle at (cx, cy) of radius r. */
const sparkle4 = (cx: number, cy: number, r: number) => {
  const a = 0.09 * r;
  const b = 0.33 * r;
  return `M${num(cx)} ${num(cy - r)}C${num(cx + a)} ${num(cy - b)} ${num(cx + b)} ${num(cy - a)} ${num(cx + r)} ${num(cy)}C${num(cx + b)} ${num(cy + a)} ${num(cx + a)} ${num(cy + b)} ${num(cx)} ${num(cy + r)}C${num(cx - a)} ${num(cy + b)} ${num(cx - b)} ${num(cy + a)} ${num(cx - r)} ${num(cy)}C${num(cx - b)} ${num(cy - a)} ${num(cx - a)} ${num(cy - b)} ${num(cx)} ${num(cy - r)}Z`;
};

/** The words on a drawn sticker: the default, its size and the widest it may get (view box units). */
interface ArtText {
  readonly value: string;
  readonly size: number;
  readonly max: number;
}
interface ArtDef {
  readonly id: string;
  readonly name: string;
  readonly keywords: string;
  readonly group: StickerGroup;
  /** The main colour ({{c}}). */
  readonly color?: string;
  readonly text?: ArtText;
  /** How it enters when added from the set (pop unless it's a stroke that draws on). */
  readonly enter?: "pop" | "draw";
  /** A 200 × 200 picture. */
  readonly body?: string;
  /** A mark: its markup for a view box `vw` wide and 200 high (the box's aspect), and its natural size. */
  readonly mark?: (vw: number) => string;
  readonly size?: readonly [number, number];
}

// ---- marks: stretch to their box ----

const H = 200;
const markStroke = (d: string, width: number, extra = "") => `<path d="${d}" fill="none" stroke="{{c}}" stroke-width="${width}" stroke-linecap="round" stroke-linejoin="round" ${DRAW}${extra}/>`;

const MARKS: ReadonlyArray<ArtDef> = [
  {
    id: "circle-hand",
    name: "Hand-drawn circle",
    keywords: "circle ring around mark highlight marker scribble",
    group: "Arrows & marks",
    color: "#FF3B30",
    enter: "draw",
    size: [560, 300],
    mark: (vw) => {
      // A loop going round a bit more than once, its radius easing in so the ends cross like a marker's.
      const cx = vw / 2;
      const cy = H / 2;
      const rx = vw / 2 - 14;
      const ry = H / 2 - 14;
      const pts: Array<readonly [number, number]> = [];
      const n = 30;
      for (let i = 0; i <= n; i++) {
        const t = i / n;
        const a = ((200 + 395 * t) * Math.PI) / 180;
        const k = 1 - 0.07 * t + 0.025 * Math.sin(t * 9);
        pts.push([cx + rx * k * Math.cos(a), cy + ry * k * Math.sin(a)]);
      }
      return markStroke(smooth(pts), 12);
    },
  },
  {
    id: "underline",
    name: "Underline",
    keywords: "line under mark emphasis marker stroke",
    group: "Arrows & marks",
    color: "#FF3B30",
    enter: "draw",
    size: [560, 80],
    mark: (vw) => {
      const pts: Array<readonly [number, number]> = [];
      for (let i = 0; i <= 8; i++) {
        const t = i / 8;
        pts.push([18 + (vw - 36) * t, H * 0.56 + 18 * Math.sin(Math.PI * t) * -1 + 16 * t]);
      }
      return markStroke(smooth(pts), 34);
    },
  },
  {
    id: "underline-double",
    name: "Double underline",
    keywords: "line under twice mark emphasis marker",
    group: "Arrows & marks",
    color: "#FF3B30",
    enter: "draw",
    size: [560, 110],
    mark: (vw) => markStroke(`M18 ${H * 0.36}Q${num(vw / 2)} ${H * 0.24} ${num(vw - 18)} ${H * 0.34}M${num(vw * 0.12)} ${H * 0.72}Q${num(vw / 2)} ${H * 0.6} ${num(vw - 30)} ${H * 0.7}`, 26),
  },
  {
    id: "highlight",
    name: "Highlighter",
    keywords: "highlight marker mark text emphasis yellow",
    group: "Arrows & marks",
    color: "#FFE14D",
    enter: "draw",
    size: [560, 120],
    mark: (vw) => `<path d="M12 ${H * 0.56}L${num(vw - 12)} ${H * 0.46}" fill="none" stroke="{{c}}" stroke-opacity="0.6" stroke-width="${H * 0.74}" stroke-linecap="butt" ${DRAW}/>`,
  },
  {
    id: "box-hand",
    name: "Hand-drawn box",
    keywords: "box frame rectangle around mark scribble",
    group: "Arrows & marks",
    color: "#FF3B30",
    enter: "draw",
    size: [560, 300],
    mark: (vw) => markStroke(`M16 26L${num(vw - 14)} 16L${num(vw - 22)} ${H - 14}L18 ${H - 22}L22 10`, 12),
  },
  {
    id: "strike",
    name: "Strike-through",
    keywords: "strike line through cross out wrong no mark",
    group: "Arrows & marks",
    color: "#FF3B30",
    enter: "draw",
    size: [560, 80],
    mark: (vw) => markStroke(`M14 ${H * 0.6}Q${num(vw / 2)} ${H * 0.44} ${num(vw - 14)} ${H * 0.4}`, 34),
  },
  {
    id: "cross-out",
    name: "Cross out",
    keywords: "cross x wrong no mark scribble cancel",
    group: "Arrows & marks",
    color: "#FF3B30",
    enter: "draw",
    size: [420, 240],
    mark: (vw) => markStroke(`M16 18Q${num(vw * 0.5)} ${H * 0.55} ${num(vw - 16)} ${H - 16}M${num(vw - 18)} 20Q${num(vw * 0.48)} ${H * 0.45} 18 ${H - 18}`, 16),
  },
  {
    id: "squiggle",
    name: "Squiggle",
    keywords: "wave wavy line under mark scribble",
    group: "Arrows & marks",
    color: "#FF3B30",
    enter: "draw",
    size: [560, 80],
    mark: (vw) => {
      const waves = Math.max(2, Math.round((vw - 40) / 130));
      const pts: Array<readonly [number, number]> = [];
      for (let i = 0; i <= waves * 4; i++) pts.push([20 + ((vw - 40) * i) / (waves * 4), H / 2 + [0, -1, 0, 1][i % 4]! * H * 0.26]);
      return markStroke(smooth(pts), 26);
    },
  },
];

// ---- pictures: 200 × 200 ----

const ink = (w = 6) => `stroke="${INK}" stroke-width="${w}" stroke-linejoin="round"`;
const SEAL = spikes(24, 94, 82);
const STARBURST = spikes(20, 96, 72);

const PICTURES: ReadonlyArray<ArtDef> = [
  // The first seven are D165's, under the same ids.
  {
    id: "burst",
    name: "Wow burst",
    keywords: "wow boom comic star explosion",
    group: "Shapes & bursts",
    color: "#FFD23F",
    text: { value: "WOW", size: 46, max: 96 },
    body: `<polygon points="100,6 119,52 168,30 152,80 196,100 152,120 168,170 119,148 100,194 81,148 32,170 48,120 4,100 48,80 32,30 81,52" fill="{{c}}" ${ink()}/>${words(100, 101, INK)}`,
  },
  { id: "arrow", name: "Arrow", keywords: "point pointer this look here curved", group: "Arrows & marks", color: "#FFFFFF", body: `<path d="M40 160 C 50 90, 100 60, 150 62" fill="none" stroke="${INK}" stroke-width="22" stroke-linecap="round"/><path d="M40 160 C 50 90, 100 60, 150 62" fill="none" stroke="{{c}}" stroke-width="12" stroke-linecap="round"/><polygon points="176,62 136,36 140,90" fill="{{c}}" ${ink()}/>` },
  { id: "heart", name: "Heart", keywords: "love like favourite favorite", group: "Shapes & bursts", color: "#FF3B5C", body: `<path d="M100 176 L 30 104 C 4 76, 20 28, 60 28 C 80 28, 94 42, 100 54 C 106 42, 120 28, 140 28 C 180 28, 196 76, 170 104 Z" fill="{{c}}" ${ink()}/><ellipse cx="62" cy="64" rx="14" ry="9" fill="#FFFFFF" opacity="0.55" transform="rotate(-30 62 64)"/>` },
  { id: "bubble", name: "Speech bubble", keywords: "talk chat say comment quote typing", group: "Bubbles", color: "#FFFFFF", body: `<path d="M30 40 H170 A18 18 0 0 1 188 58 V122 A18 18 0 0 1 170 140 H86 L52 172 L58 140 H30 A18 18 0 0 1 12 122 V58 A18 18 0 0 1 30 40 Z" fill="{{c}}" ${ink()}/><circle cx="64" cy="90" r="10" fill="${INK}"/><circle cx="100" cy="90" r="10" fill="${INK}"/><circle cx="136" cy="90" r="10" fill="${INK}"/>` },
  {
    id: "new",
    name: "New badge",
    keywords: "new label tag fresh latest",
    group: "Badges",
    color: "#FF3B5C",
    text: { value: "NEW", size: 54, max: 140 },
    body: `<g transform="rotate(-8 100 100)"><rect x="14" y="58" width="172" height="84" rx="22" fill="{{c}}" ${ink()}/>${words(100, 101, "#FFFFFF")}</g>`,
  },
  { id: "sparkle", name: "Sparkle", keywords: "shine magic star twinkle clean", group: "Shapes & bursts", color: "#FFE14D", body: `<path d="M100 10 C 108 70, 130 92, 190 100 C 130 108, 108 130, 100 190 C 92 130, 70 108, 10 100 C 70 92, 92 70, 100 10 Z" fill="{{c}}" ${ink()}/><path d="M160 22 C 163 38, 168 43, 184 46 C 168 49, 163 54, 160 70 C 157 54, 152 49, 136 46 C 152 43, 157 38, 160 22 Z" fill="#FFFFFF" ${ink(4)}/>` },
  { id: "check", name: "Check", keywords: "done yes correct tick ok", group: "Shapes & bursts", color: "#3DDC84", body: `<circle cx="100" cy="100" r="84" fill="{{c}}" ${ink()}/><path d="M58 102 L 88 132 L 144 72" fill="none" stroke="#FFFFFF" stroke-width="20" stroke-linecap="round" stroke-linejoin="round" ${DRAW}/>` },

  // Arrows and hand-drawn marks.
  { id: "arrow-hand", name: "Hand-drawn arrow", keywords: "arrow point pointer this look here marker scribble", group: "Arrows & marks", color: "#FF3B30", enter: "draw", body: `<path d="M28 170 C 40 110, 84 66, 160 52" fill="none" stroke="{{c}}" stroke-width="13" stroke-linecap="round" ${DRAW}/><path d="M128 30 L164 51 L138 84" fill="none" stroke="{{c}}" stroke-width="13" stroke-linecap="round" stroke-linejoin="round" opacity="{{f}}"/>` },
  { id: "arrow-loop", name: "Loop arrow", keywords: "arrow loop curly point pointer this look here", group: "Arrows & marks", color: "#FFFFFF", enter: "draw", body: `<path d="M18 156 C 60 156, 92 134, 94 102 C 96 70, 58 64, 54 94 C 50 124, 100 134, 130 110 C 150 94, 160 80, 168 62" fill="none" stroke="${INK}" stroke-width="22" stroke-linecap="round" ${DRAW}/><path d="M18 156 C 60 156, 92 134, 94 102 C 96 70, 58 64, 54 94 C 50 124, 100 134, 130 110 C 150 94, 160 80, 168 62" fill="none" stroke="{{c}}" stroke-width="11" stroke-linecap="round" ${DRAW}/><path d="M140 74 L168 60 L176 92" fill="none" stroke="${INK}" stroke-width="22" stroke-linecap="round" stroke-linejoin="round" opacity="{{f}}"/><path d="M140 74 L168 60 L176 92" fill="none" stroke="{{c}}" stroke-width="11" stroke-linecap="round" stroke-linejoin="round" opacity="{{f}}"/>` },
  { id: "arrow-down", name: "Arrow down", keywords: "arrow down below link description point", group: "Arrows & marks", color: "#FFFFFF", enter: "draw", body: `<path d="M104 18 C 70 60, 134 96, 100 158" fill="none" stroke="${INK}" stroke-width="22" stroke-linecap="round" ${DRAW}/><path d="M104 18 C 70 60, 134 96, 100 158" fill="none" stroke="{{c}}" stroke-width="11" stroke-linecap="round" ${DRAW}/><path d="M72 134 L100 168 L130 138" fill="none" stroke="${INK}" stroke-width="22" stroke-linecap="round" stroke-linejoin="round" opacity="{{f}}"/><path d="M72 134 L100 168 L130 138" fill="none" stroke="{{c}}" stroke-width="11" stroke-linecap="round" stroke-linejoin="round" opacity="{{f}}"/>` },
  { id: "arrow-block", name: "Block arrow", keywords: "arrow right next point bold", group: "Arrows & marks", color: "#FFD23F", body: `<polygon points="16,76 112,76 112,36 186,100 112,164 112,124 16,124" fill="{{c}}" ${ink()}/><path d="M26 86 H104" stroke="#FFFFFF" stroke-opacity="0.55" stroke-width="7" stroke-linecap="round"/>` },
  { id: "tick", name: "Tick", keywords: "check done yes correct ok marker hand", group: "Arrows & marks", color: "#3DDC84", enter: "draw", body: `<path d="M34 106 C 50 120, 64 136, 78 154 C 102 106, 134 66, 172 38" fill="none" stroke="{{c}}" stroke-width="20" stroke-linecap="round" stroke-linejoin="round" ${DRAW}/>` },
  { id: "x-hand", name: "Hand-drawn X", keywords: "cross wrong no cancel marker x", group: "Arrows & marks", color: "#FF3B30", enter: "draw", body: `<path d="M44 42 C 80 84, 120 124, 158 160 M158 40 C 120 80, 84 120, 44 162" fill="none" stroke="{{c}}" stroke-width="20" stroke-linecap="round" ${DRAW}/>` },
  { id: "emphasis", name: "Emphasis lines", keywords: "lines pop shock attention look action", group: "Arrows & marks", color: "#FFD23F", enter: "draw", body: `<path d="M74 70 L40 30 M104 54 L100 10 M58 102 L14 94 M126 146 L160 186 M96 162 L100 196 M142 118 L186 126" fill="none" stroke="${INK}" stroke-width="20" stroke-linecap="round" ${DRAW}/><path d="M74 70 L40 30 M104 54 L100 10 M58 102 L14 94 M126 146 L160 186 M96 162 L100 196 M142 118 L186 126" fill="none" stroke="{{c}}" stroke-width="10" stroke-linecap="round" ${DRAW}/>` },
  { id: "question", name: "Question mark", keywords: "question what why ask hmm confused", group: "Arrows & marks", color: "#FFD23F", enter: "draw", body: `<path d="M62 70 C 60 28, 142 24, 142 70 C 142 100, 100 102, 100 134" fill="none" stroke="${INK}" stroke-width="30" stroke-linecap="round" ${DRAW}/><path d="M62 70 C 60 28, 142 24, 142 70 C 142 100, 100 102, 100 134" fill="none" stroke="{{c}}" stroke-width="18" stroke-linecap="round" ${DRAW}/><circle cx="100" cy="170" r="13" fill="{{c}}" ${ink()} opacity="{{f}}"/>` },

  // Shapes and bursts.
  { id: "star", name: "Star", keywords: "star rating favourite favorite best gold", group: "Shapes & bursts", color: "#FFC531", body: `<polygon points="100,16 122.3,73.3 183.7,76.8 136.1,115.7 151.7,175.2 100,142 48.3,175.2 63.9,115.7 16.3,76.8 77.7,73.3" fill="{{c}}" ${ink()}/><ellipse cx="82" cy="88" rx="11" ry="6" fill="#FFFFFF" opacity="0.6" transform="rotate(-35 82 88)"/>` },
  {
    id: "boom",
    name: "Boom",
    keywords: "boom bang pow explosion comic blast",
    group: "Shapes & bursts",
    color: "#FF7A1A",
    text: { value: "BOOM", size: 40, max: 100 },
    body: `<polygon points="100,8 118,44 150,18 148,58 192,52 162,88 196,118 156,124 172,170 128,148 112,192 92,152 54,186 58,140 10,150 38,112 6,78 48,72 30,30 76,46" fill="{{c}}" ${ink()}/><polygon points="100,33.8 113,59.7 136,41 134.6,69.8 166.2,65.4 144.6,91.4 169.1,113 140.3,117.3 151.8,150.4 120.2,134.6 108.6,166.2 94.2,137.4 66.9,161.9 69.8,128.8 35.2,136 55.4,108.6 32.3,84.2 62.6,79.8 49.6,49.6 82.7,61.1" fill="#FFE14D"/>${words(100, 101, INK)}`,
  },
  { id: "starburst", name: "Starburst", keywords: "burst star spiky flash price deal", group: "Shapes & bursts", color: "#FF3B5C", text: { value: "!", size: 92, max: 120 }, body: `<polygon points="${STARBURST}" fill="{{c}}" ${ink()}/>${words(100, 104, "#FFFFFF")}` },
  { id: "twinkles", name: "Twinkles", keywords: "sparkle sparkles shine magic stars clean new", group: "Shapes & bursts", color: "#FFFFFF", body: `<path d="${sparkle4(72, 112, 58)}" fill="{{c}}" ${ink(5)}/><path d="${sparkle4(150, 54, 32)}" fill="{{c}}" ${ink(5)}/><path d="${sparkle4(150, 152, 22)}" fill="{{c}}" ${ink(5)}/>` },
  { id: "lightning", name: "Lightning bolt", keywords: "lightning fast power energy zap flash", group: "Shapes & bursts", color: "#FFD23F", body: `<polygon points="122,8 38,116 92,116 72,194 166,78 110,78 136,8" fill="{{c}}" ${ink()}/><path d="M112 26 L62 98" stroke="#FFFFFF" stroke-opacity="0.6" stroke-width="7" stroke-linecap="round"/>` },
  { id: "cross", name: "Cross", keywords: "no wrong cancel x stop", group: "Shapes & bursts", color: "#FF3B30", body: `<circle cx="100" cy="100" r="84" fill="{{c}}" ${ink()}/><path d="M68 68 L132 132 M132 68 L68 132" fill="none" stroke="#FFFFFF" stroke-width="20" stroke-linecap="round" ${DRAW}/>` },
  { id: "blob", name: "Blob", keywords: "blob shape organic backdrop spot", group: "Shapes & bursts", color: "#7C5CFF", body: `<path d="M100 18 C 150 14, 190 50, 184 104 C 178 158, 136 190, 92 184 C 44 178, 12 140, 18 92 C 24 48, 56 22, 100 18 Z" fill="{{c}}"/>` },
  {
    id: "confetti",
    name: "Confetti",
    keywords: "confetti celebrate party congrats hooray",
    group: "Shapes & bursts",
    body: `<rect x="30" y="40" width="24" height="11" rx="2" fill="#FF3B5C" transform="rotate(30 42 45)"/><circle cx="150" cy="40" r="9" fill="#FFD23F"/><rect x="118" y="88" width="26" height="11" rx="2" fill="#3DDC84" transform="rotate(-40 131 93)"/><circle cx="58" cy="122" r="8" fill="#4DA3FF"/><rect x="146" y="146" width="22" height="10" rx="2" fill="#FF8A00" transform="rotate(60 157 151)"/><circle cx="100" cy="172" r="7" fill="#FF3B5C"/><path d="M70 72 q 8 -14 16 0 t 16 0" fill="none" stroke="#7C5CFF" stroke-width="7" stroke-linecap="round"/><rect x="28" y="160" width="20" height="9" rx="2" fill="#FFD23F" transform="rotate(-20 38 164)"/><circle cx="178" cy="104" r="7" fill="#3DDC84"/><rect x="92" y="22" width="20" height="9" rx="2" fill="#4DA3FF" transform="rotate(15 102 26)"/><path d="M128 178 q 6 -12 12 0 t 12 0" fill="none" stroke="#FF3B5C" stroke-width="6" stroke-linecap="round"/>`,
  },

  // Bubbles.
  { id: "speech", name: "Talk bubble", keywords: "speech bubble talk say hi hello chat quote", group: "Bubbles", color: "#FFFFFF", text: { value: "Hi!", size: 52, max: 140 }, body: `<path d="M100 26 C 154 26, 188 54, 188 92 C 188 130, 154 158, 100 158 C 88 158, 76 156, 66 153 L 30 180 L 40 142 C 22 129, 12 112, 12 92 C 12 54, 46 26, 100 26 Z" fill="{{c}}" ${ink()}/>${words(100, 92, INK)}` },
  { id: "thought", name: "Thought bubble", keywords: "think thought hmm idea wonder dream", group: "Bubbles", color: "#FFFFFF", text: { value: "Hmm", size: 42, max: 116 }, body: `<circle cx="26" cy="186" r="7" fill="{{c}}" ${ink(5)}/><circle cx="46" cy="164" r="11" fill="{{c}}" ${ink(5)}/><path d="M62 142 C 30 144, 14 118, 30 98 C 14 76, 34 46, 62 54 C 70 28, 112 22, 126 44 C 150 30, 186 50, 176 78 C 196 96, 184 134, 154 132 C 148 152, 112 158, 100 142 C 88 156, 66 154, 62 142 Z" fill="{{c}}" ${ink()}/>${words(104, 92, INK)}` },
  { id: "shout", name: "Shout bubble", keywords: "shout yell omg wow comic exclaim loud", group: "Bubbles", color: "#FFE14D", text: { value: "OMG", size: 44, max: 108 }, body: `<polygon points="20,60 46,52 50,22 80,44 104,14 120,46 156,24 154,58 190,64 164,90 192,118 156,122 160,156 124,140 112,168 92,142 40,188 58,136 18,140 36,104 8,84" fill="{{c}}" ${ink()}/>${words(100, 94, INK)}` },
  { id: "tip", name: "Tip box", keywords: "tip callout note box hint pro advice", group: "Bubbles", color: "#1C1C1F", text: { value: "Pro tip", size: 38, max: 150 }, body: `<path d="M24 34 H176 A14 14 0 0 1 190 48 V124 A14 14 0 0 1 176 138 H72 L40 174 L46 138 H24 A14 14 0 0 1 10 124 V48 A14 14 0 0 1 24 34 Z" fill="{{c}}" stroke="#FFFFFF" stroke-width="5" stroke-linejoin="round"/>${words(100, 86, "#FFFFFF")}` },
  { id: "chat", name: "Chat", keywords: "chat message messages text dm conversation typing", group: "Bubbles", color: "#2F80FF", body: `<path d="M44 34 H112 A30 30 0 0 1 112 94 H52 L28 110 L32 88 A30 30 0 0 1 44 34 Z" fill="#E9E9EB" ${ink(5)}/><circle cx="56" cy="64" r="7" fill="#8E8E93"/><circle cx="78" cy="64" r="7" fill="#8E8E93"/><circle cx="100" cy="64" r="7" fill="#8E8E93"/><path d="M96 104 H156 A30 30 0 0 1 168 162 L172 180 L150 164 H96 A30 30 0 0 1 96 104 Z" fill="{{c}}" ${ink(5)}/><path d="M92 124 H146 M92 144 H128" stroke="#FFFFFF" stroke-width="8" stroke-linecap="round"/>` },

  // Badges.
  { id: "sale", name: "Sale tag", keywords: "sale price discount deal offer tag shop", group: "Badges", color: "#FF3B30", text: { value: "SALE", size: 44, max: 90 }, body: `<g transform="rotate(-6 100 100)"><path d="M30 52 H148 L186 100 L148 148 H30 A14 14 0 0 1 16 134 V66 A14 14 0 0 1 30 52 Z" fill="{{c}}" ${ink()}/><circle cx="154" cy="100" r="9" fill="#FFFFFF" ${ink(4)}/>${words(82, 101, "#FFFFFF")}</g>` },
  { id: "hot", name: "Hot badge", keywords: "hot trending popular deal flame", group: "Badges", color: "#FF6A00", text: { value: "HOT", size: 46, max: 96 }, body: `<rect x="12" y="58" width="176" height="84" rx="42" fill="{{c}}" ${ink()}/><path d="M48 72 C 66 88, 72 102, 62 118 C 58 126, 40 126, 36 116 C 32 104, 42 98, 44 88 C 48 94, 50 96, 52 92 C 54 86, 52 78, 48 72 Z" fill="#FFE14D" ${ink(4)}/>${words(118, 101, "#FFFFFF")}` },
  { id: "free", name: "Free badge", keywords: "free gift giveaway offer deal", group: "Badges", color: "#3DDC84", text: { value: "FREE", size: 44, max: 128 }, body: `<polygon points="${STARBURST}" fill="{{c}}" ${ink()}/>${words(100, 101, INK)}` },
  { id: "live", name: "Live badge", keywords: "live stream streaming now on air broadcast", group: "Badges", color: "#FF2D55", text: { value: "LIVE", size: 44, max: 108 }, body: `<rect x="12" y="62" width="176" height="76" rx="16" fill="{{c}}" ${ink()}/><circle cx="46" cy="100" r="11" fill="#FFFFFF"/>${words(116, 101, "#FFFFFF")}` },
  { id: "seal", name: "Discount seal", keywords: "seal discount percent off deal sale offer stamp", group: "Badges", color: "#FFD23F", text: { value: "-50%", size: 50, max: 104 }, body: `<polygon points="${SEAL}" fill="{{c}}" ${ink()}/><circle cx="100" cy="100" r="66" fill="none" stroke="${INK}" stroke-width="3" stroke-dasharray="7 7"/>${words(100, 101, INK)}` },
  { id: "ribbon", name: "Ribbon", keywords: "ribbon banner best top pick award winner label", group: "Badges", color: "#7C5CFF", text: { value: "BEST", size: 40, max: 124 }, body: `<polygon points="8,88 48,88 48,144 8,144 24,116" fill="{{c}}" ${ink()}/><polygon points="192,88 152,88 152,144 192,144 176,116" fill="{{c}}" ${ink()}/><polygon points="8,88 48,88 48,144 8,144 24,116" fill="#000000" opacity="0.28"/><polygon points="192,88 152,88 152,144 192,144 176,116" fill="#000000" opacity="0.28"/><rect x="30" y="64" width="140" height="64" fill="{{c}}" ${ink()}/>${words(100, 97, "#FFFFFF")}` },
  { id: "before", name: "Before label", keywords: "before after comparison then old label tag", group: "Badges", color: "#FFFFFF", text: { value: "BEFORE", size: 40, max: 156 }, body: `<rect x="10" y="64" width="180" height="72" rx="12" fill="{{c}}" ${ink()}/>${words(100, 101, INK)}` },
  { id: "after", name: "After label", keywords: "after before comparison now new result label tag", group: "Badges", color: "#3DDC84", text: { value: "AFTER", size: 40, max: 156 }, body: `<rect x="10" y="64" width="180" height="72" rx="12" fill="{{c}}" ${ink()}/>${words(100, 101, INK)}` },
  { id: "soon", name: "Coming soon", keywords: "soon coming tape label teaser upcoming", group: "Badges", color: "#FFE14D", text: { value: "SOON", size: 46, max: 160 }, body: `<g transform="rotate(-4 100 100)"><polygon points="6,66 194,60 190,82 196,104 192,126 194,140 8,146 12,124 6,104 12,84" fill="{{c}}"/>${words(100, 103, INK)}</g>` },

  // Social (generic glyphs we drew; no platform logos).
  { id: "like", name: "Like", keywords: "like thumbs up approve good social", group: "Social", color: "#2F80FF", body: `<circle cx="100" cy="100" r="86" fill="{{c}}" ${ink()}/><path d="M58 96 H80 V150 H58 Z" fill="#FFFFFF" stroke="#FFFFFF" stroke-width="5" stroke-linejoin="round"/><path d="M90 150 V96 L112 52 C 117 43, 133 46, 131 61 L 127 84 H150 C 161 84, 167 95, 163 105 L 153 141 C 151 147, 145 150, 139 150 Z" fill="#FFFFFF" stroke="#FFFFFF" stroke-width="5" stroke-linejoin="round"/>` },
  { id: "love", name: "Love", keywords: "love heart like favourite favorite social", group: "Social", color: "#FF3B5C", body: `<circle cx="100" cy="100" r="86" fill="{{c}}" ${ink()}/><path d="M100 150 L 62 112 C 48 98, 56 70, 78 70 C 88 70, 96 76, 100 84 C 104 76, 112 70, 122 70 C 144 70, 152 98, 138 112 Z" fill="#FFFFFF"/>` },
  { id: "bell", name: "Bell", keywords: "bell notification notify alert subscribe social", group: "Social", color: "#FFD23F", body: `<circle cx="100" cy="160" r="15" fill="{{c}}" ${ink()}/><path d="M100 26 C 138 26, 150 56, 150 92 V124 L168 148 H32 L50 124 V92 C 50 56, 62 26, 100 26 Z" fill="{{c}}" ${ink()}/><path d="M72 56 C 66 66, 64 78, 64 92" fill="none" stroke="#FFFFFF" stroke-opacity="0.65" stroke-width="8" stroke-linecap="round"/><circle cx="152" cy="44" r="17" fill="#FF3B30" ${ink(5)}/>` },
  { id: "play", name: "Play button", keywords: "play video watch start button social", group: "Social", color: "#FF3B5C", body: `<circle cx="100" cy="100" r="86" fill="{{c}}" ${ink()}/><polygon points="82,62 82,138 144,100" fill="#FFFFFF" stroke="#FFFFFF" stroke-width="10" stroke-linejoin="round"/>` },
  { id: "comment", name: "Comment", keywords: "comment reply message talk social", group: "Social", color: "#FFFFFF", body: `<path d="M100 28 C 154 28, 186 58, 186 96 C 186 134, 154 162, 100 162 C 86 162, 74 160, 62 156 L 24 178 L 36 140 C 20 128, 14 112, 14 96 C 14 58, 46 28, 100 28 Z" fill="{{c}}" ${ink()}/><path d="M58 82 H142 M58 110 H118" stroke="${INK}" stroke-width="11" stroke-linecap="round"/>` },
  { id: "share", name: "Share", keywords: "share send forward repost social", group: "Social", color: "#3DDC84", body: `<path d="M110 36 L186 98 L110 160 V122 C 68 120, 40 136, 16 170 C 22 114, 50 78, 110 72 Z" fill="{{c}}" ${ink()}/>` },
  { id: "save", name: "Save", keywords: "save bookmark keep later social", group: "Social", color: "#FFD23F", body: `<path d="M56 22 H144 A10 10 0 0 1 154 32 V180 L100 142 L46 180 V32 A10 10 0 0 1 56 22 Z" fill="{{c}}" ${ink()}/>` },
  { id: "follow", name: "Follow", keywords: "follow add friend person user plus social", group: "Social", color: "#7C5CFF", body: `<circle cx="100" cy="100" r="86" fill="{{c}}" ${ink()}/><circle cx="86" cy="80" r="22" fill="#FFFFFF"/><path d="M46 146 C 46 118, 64 106, 86 106 C 108 106, 126 118, 126 146 Z" fill="#FFFFFF"/><path d="M152 70 V110 M132 90 H172" stroke="#FFFFFF" stroke-width="12" stroke-linecap="round"/>` },
  { id: "views", name: "Views", keywords: "views eye watch look seen social", group: "Social", color: "#2F80FF", body: `<path d="M12 100 C 44 48, 156 48, 188 100 C 156 152, 44 152, 12 100 Z" fill="#FFFFFF" ${ink()}/><circle cx="100" cy="100" r="34" fill="{{c}}" ${ink(5)}/><circle cx="100" cy="100" r="14" fill="${INK}"/><circle cx="111" cy="89" r="6" fill="#FFFFFF"/>` },
  { id: "cursor", name: "Click", keywords: "click cursor pointer mouse tap press", group: "Social", color: "#FFFFFF", body: `<path d="M48 22 L36 6 M34 44 L14 42 M66 16 L72 -2" stroke="${INK}" stroke-width="7" stroke-linecap="round" transform="translate(0 6)"/><path d="M62 34 L62 164 L94 134 L116 182 L140 172 L118 126 L162 126 Z" fill="{{c}}" ${ink()}/>` },
];

const ART: ReadonlyArray<ArtDef> = [...PICTURES.slice(0, 7), ...MARKS, ...PICTURES.slice(7)];
const ART_BY_ID = new Map(ART.map((a) => [a.id, a]));

/** How wide words are in DM Sans Black, in ems (per-character estimates, so both renderers agree without measuring). */
export function blackWidth(text: string): number {
  let w = 0;
  for (const ch of text) {
    if (ch === "W" || ch === "M") w += 0.98;
    else if (ch === "m" || ch === "w") w += 0.86;
    else if (/[A-Z%]/.test(ch)) w += 0.74;
    else if (/[0-9$€£]/.test(ch)) w += 0.64;
    else if (/[a-z]/.test(ch)) w += 0.58;
    else if (ch === " ") w += 0.26;
    else w += 0.4;
  }
  return w;
}
const clamp01 = (v: number) => Math.min(1, Math.max(0, v));
/** A valid CSS-ish colour for the art: user values come through resolveColor; refuse anything that could break out of the attribute. */
const safeColor = (c: string | undefined, fallback: string) => (c && /^[#a-zA-Z0-9(),.%\s-]+$/.test(c) ? c : fallback);

/**
 * A drawn sticker's SVG. `color` replaces its main colour, `text` its words, `draw` (0–1) how much
 * of it is drawn (strokes first, then the parts that come after), `aspect` (w / h) the box a mark
 * stretches to (pictures are always square). Deterministic and self-contained.
 */
export function artSvg(id: string, opts: { color?: string; text?: string; draw?: number; aspect?: number } = {}): string {
  const a = ART_BY_ID.get(id);
  if (!a) return "";
  const p = clamp01(opts.draw ?? 1);
  const dash = num(1 - clamp01(p / 0.8));
  const after = num(clamp01((p - 0.7) / 0.3));
  const raw = opts.text ?? a.text?.value ?? "";
  const shown = [...raw].slice(0, 24).join("");
  const size = a.text ? Math.min(a.text.size, a.text.max / Math.max(0.5, blackWidth(shown))) : 0;
  const naturalAspect = a.size ? a.size[0] / a.size[1] : 1;
  const vw = a.mark ? Math.max(H * 0.2, H * (opts.aspect && Number.isFinite(opts.aspect) && opts.aspect > 0 ? opts.aspect : naturalAspect)) : 200;
  const body = (a.mark ? a.mark(vw) : a.body!)
    .replaceAll("{{c}}", safeColor(opts.color, a.color ?? "#FFFFFF"))
    .replaceAll("{{d}}", dash)
    .replaceAll("{{f}}", after)
    .replaceAll("{{ts}}", num(size))
    .replaceAll("{{t}}", esc(shown));
  const fit = a.mark ? ` preserveAspectRatio="none" overflow="visible"` : "";
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${num(vw)} ${H}" width="100%" height="100%"${fit}>${body}</svg>`;
}

/** A drawn sticker's info: whether it's a mark (stretches to its box), its natural size (manifest px), its words. */
export function stickerArt(id: unknown): { readonly mark: boolean; readonly size?: readonly [number, number]; readonly text?: string; readonly color?: string } | undefined {
  const a = typeof id === "string" ? ART_BY_ID.get(id) : undefined;
  return a && { mark: !!a.mark, size: a.size, text: a.text?.value, color: a.color };
}

export const STICKER_ART: Readonly<Record<string, { readonly name: string; readonly svg: string; readonly group: StickerGroup; readonly mark: boolean }>> = Object.fromEntries(ART.map((a) => [a.id, { name: a.name, svg: artSvg(a.id), group: a.group, mark: !!a.mark }]));
export const STICKER_ART_IDS = ART.map((a) => a.id);

const REACTIONS = EMOJI.findIndex(([e]) => e === "💯");
/** Every sticker: the drawn ones first, then the emoji. */
export const STICKERS: ReadonlyArray<StickerEntry> = [
  ...ART.map((a) => ({ key: `art:${a.id}`, name: a.name, keywords: a.keywords.split(" "), group: a.group, art: a.id })),
  ...EMOJI.map(([emoji, name, keywords], i) => ({ key: `emoji:${emoji}`, name, keywords: keywords.split(" "), group: (i <= REACTIONS ? "Reactions" : "Emoji") as StickerGroup, emoji })),
];

/** The props a sticker from the set gets (component.sticker): the emoji, or the drawn art and how it enters. */
export const stickerProps = (entry: StickerEntry): Record<string, unknown> => (entry.art ? { art: entry.art, enter: ART_BY_ID.get(entry.art)?.enter ?? "pop", tilt: 0 } : { emoji: entry.emoji!, enter: "pop", tilt: -8 });
