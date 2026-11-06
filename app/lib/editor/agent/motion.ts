// Adapted from Uncut criticMotion.ts, MIT; Node process decoding is hosted in Rust.
// The critic sees motion, not just stills (D292). Still frames can't show a single-frame pop, a
// move that lands off the beat, a hold that drags or a loop that stutters at its seam, so a review
// round also gets:
// - frame strips: for each scene (the storyboard's beats, else evenly spaced moments), a row of N
//   frames around its start at 1/12 s apart, downscaled, one image per strip (bounded: at most
//   MAX_STRIPS strips of STRIP_FRAMES cells, STRIP_CELL px wide);
// - a frame-difference scan: small grey frames (SCAN_WIDTH px) of consecutive frames around each
//   scene change plus an even sampling of the whole film (at most SCAN_BUDGET frames, rendered once
//   with the strips' at STRIP_CELL and stopped at MOTION_BUDGET_MS), from which
//   it finds single-frame pops (a frame that differs from both neighbours more than POP_RATIO times
//   as much as they differ from each other), one-frame jumps (a cut, or a flood that took one
//   frame), holds over a second and the loop seam (last frame against frame 0).
// What it finds goes to the critic as facts (<motion>) and to the agent as review findings
// (rule "motion"). Pure planning and scanning here; the rendering is review_submit's (cards.ts),
// and grayFrames decodes the rendered JPEGs with ffmpeg (no image library in the repo).


import type { CriticFinding } from "./critic";
import { CAPTION_HEIGHT, GAP, type SheetLayout } from "./sheets";

/** Frames in a strip, their spacing (s), the most strips, a cell's width (px). */
export const STRIP_FRAMES = 6;
export const STRIP_STEP_S = 1 / 12;
export const MAX_STRIPS = 6;
export const STRIP_CELL = 200;
/** The scan's grey frames are this wide; at most this many frames are rendered for it. */
export const SCAN_WIDTH = 64;
export const SCAN_RENDER_WIDTH = 96;
/**
 * The most time (ms) review_submit spends rendering the strips' and the scan's frames: past it the
 * scan's remaining frames are left out and the scan runs on what was rendered (and says so). The
 * strips' frames and the scene changes' windows come first, so a cut-off scan still covers them.
 */
export const MOTION_BUDGET_MS = 40_000;
export const SCAN_BUDGET = 240;
/** Consecutive frames on each side of a scene change in the scan. */
export const SCAN_WINDOW = 5;
/** A pop or a jump: this many times its neighbours' difference, and at least POP_MIN (mean |Δ| of 0..1 grey). */
export const POP_RATIO = 3;
export const POP_MIN = 0.03;
/**
 * A flash: a run of 2 to FLASH_S worth of frames unlike the frames on both sides, which agree with
 * each other (the picture returns), its way in and out each POP_RATIO times the return and the run's
 * own steps. A1r8 (12.98 and 15.48 s): the next shot full-bleed for two frames before its grow began
 * (in 19.6 and 25.6 %, out the same, return 0.4 and 0.2 %, still inside); a pop needs one frame and a
 * jump a picture that stays, so neither saw it. None in the references (2.mp4, 3.mp4) or in the
 * earlier films A1 to A2r5, B1, B2.
 */
export const FLASH_S = 0.05;
/** A jump is also at least this many times the busier side's biggest step (a cut into a fast move). */
export const JUMP_OVER = 1.5;
/**
 * Round 4's A1 (18.05 s): footage swapped to the agent panel over 4 frames (steps 2.6, 4.9, 8.7,
 * 8.6 %, then 1 %), a cut to the eye that no one-frame rule sees. A quick swap: a run of 2 to
 * BURST_S worth of steps each at least a quarter of its peak, the peak BURST_PEAK or more, the
 * two steps on each side of it under that quarter (the move starts and stops inside 0.09 s) and
 * the picture going from one state to another (BURST_NET). The
 * designed grows of the same film peak at 3 to 4 % over 7 to 10 frames; plain Claude's films had none.
 */
export const BURST_S = 0.09;
export const BURST_PEAK = 0.05;
/** And its net change at least this share of its steps' sum (A1r4's swap 0.92, a zoom snap 0.64). */
export const BURST_NET = 0.6;
/** Below this mean difference two samples count as the same picture (a hold). */
export const HOLD_DIFF = 0.0015;
export const HOLD_MAX_S = 1;
/**
 * A nearly empty frame (emptyMeasure, at EMPTY_WIDTH): what isn't the canvas reaching the frame's
 * edge is at most EMPTY_CONTENT of it, in at most EMPTY_PARTS separate marks: a cursor (and its
 * shadow or click ring), a speck. Rehearsal 4: A1 22.1-22.5 s (cursor only), A2 4.2-4.4 s. The review
 * of 72a73b9: a canvas share alone (97.5 % at 64 px) flagged a dark full-bleed UI (soft panels on
 * near-black read as canvas, its text 1.5 %) and sat close to a lone wordmark (96 %); text is many
 * marks, so a dark UI's lines, a wordmark's letters and a headline aren't empty.
 */
export const EMPTY_CONTENT = 0.015;
export const EMPTY_PARTS = 3;
/** Emptiness is judged on grey frames this wide (the scan's 64 px blur a wordmark into a speck). */
export const EMPTY_WIDTH = 160;
/** A sample this bare or barer (content share) is a suspect: the gap to its neighbours is sampled at most EMPTY_STEP_S apart. */
export const EMPTY_SUSPECT = 0.08;
export const EMPTY_STEP_S = 0.15;
/** A nearly empty run this long or longer is a finding. */
export const EMPTY_MIN_S = 0.2;
/**
 * A nearly empty run this long or longer is a blocker in a motion film. Measured frame by frame at
 * 60 fps: the original target, A1 22.1 s (cursor only), 0.42 s; B2b 0.38 s, T2b 0.55 s; the reference
 * 2.mp4's black bleed at 21.9 s 0.27 s and the favourite A2's Send at 4.2 s 0.18 to 0.2 s (sampled),
 * which the old 0.2 s bar blocked. Shorter runs from EMPTY_MIN_S stay findings.
 */
export const EMPTY_HARD_S = 1 / 3;
/**
 * A low-information frame (thinMeasure, at THIN_WIDTH on the canvas's short side: the strips' own
 * size for a portrait or square canvas, so a 1.5 % line keeps its height; thinSize): THIN_FLAT or more of it one flat colour family (± THIN_LEVELS: a dark panel on the
 * near-black canvas and its card read as one field), with readable marks (THIN_CONTRAST off that
 * colour, letters joined into words) but none of them wider than a cursor (THIN_MARK_W) and taller
 * than THIN_TALL of the frame: a big dark panel with a line or two of desktop-size text. The empty
 * rule counts that panel as content (grey 17 on 10 is more than FLAT_LEVELS), so T1 4.6 to 6.2 s and
 * T1b 4.6 to 6.4 s (the agent panel after Send, its lines 1.5 % of the height) passed it. Measured
 * at 200 px every 0.05 s, the longest run: T1 1.35 s, T1b 1.8 s, A1r8 0.75 s (steps at 2 %); the
 * favourites A2-noskill 0.15 s (its bubble lines are 3.2 % tall), A2 and A1-noskill 0.25 s (a pulled-
 * back editor); 2.mp4 and 3.mp4 0.05 s at most (their bare moments, a dot growing or a black bleed,
 * carry no mark at all: the empty rule's). Frame by frame at 60 fps: T1 1.35 s, T1b 1.82 s, A1r8
 * 0.75 s; A2 0.27 s, A1-noskill 0.18 s, A2-noskill 0.15 s. THIN_TALL is a share of the height and
 * THIN_MARK_W of the short side, so a 16:9 or 9:16 canvas is judged like a 4:5 one.
 */
export const THIN_WIDTH = 200;
export const THIN_FLAT = 0.9;
export const THIN_LEVELS = 20;
export const THIN_CONTRAST = 40;
export const THIN_MARK_W = 0.06;
export const THIN_TALL = 0.025;
/** A low-information run this long is a blocker in a motion film; from EMPTY_MIN_S it is a finding. */
export const THIN_HARD_S = 0.5;
/**
 * One still frame this long in a motion film is a finding (never a blocker): A1r9 and A1-noskill3
 * held their end card 4.9 to 5.0 s. Measured as the scan's holds: 2.mp4 1.1 s, 3.mp4 1.5 s,
 * A2-noskill 3.45 s (its slow typing open) and A1-noskill 3.38 s (its end card), A1-noskill2 3.35 s.
 */
export const STILL_MAX_S = 4;
/**
 * A fast move: at least FAST_RUN consecutive frames each changing FAST_STEP or more of the picture.
 * The export's motion blur draws it from a few sub-frames, so it stacks into copies (A2 14.0 and
 * 17.0 s: two halves sliding across the frame in a few frames read as a pixelated glitch); render
 * frames draw no blur, so neither the agent nor the critic sees it.
 */
export const FAST_STEP = 0.06;
export const FAST_RUN = 3;
/**
 * The open: the first real change is OPEN_MOVE or more of the picture changing between two samples
 * about OPEN_WINDOW_S apart. Blind round 3: the references' first change lands at 0.58 to 0.6 s (a
 * headline folding into a pill, a button reacting to the cursor); the films judged static for their
 * first 2 s land at 1.5 to 1.9 s, though a slow camera drift ran under them (under 1 % per window).
 * Later than OPEN_MAX_S is a finding in a one-take film.
 */
export const OPEN_MOVE = 0.015;
export const OPEN_WINDOW_S = 0.15;
export const OPEN_MAX_S = 1;
/**
 * D306: typing is a change too. The user's favourite open (A1-noskill: the editor blurred behind a
 * sharp composer card, the prompt typed from 0.1 s) changes about 1 % of the 64 px grey per window,
 * under OPEN_MOVE, so it read as a static open and its 3.3 s of typing as a hold. On the fine grey
 * (thinSize, 200 px on the short side) a typed word is a compact patch of strong change (THIN_CONTRAST
 * or more): at least TYPE_INK of the picture, inside a box of at most TYPE_BOX of it (a drift moves
 * every edge: A1r2's spans 0.24), still there TYPE_KEEP of it a window later (a blink or a gliding
 * cursor goes back or moves on; A1's cursor glide kept 0.71). Typing that starts by OPEN_TYPE_S
 * makes the open move. Measured every frame and every 5th and 9th: A1-noskill 0.45 to 0.5 s,
 * A1-noskill2, 4 and 5 0.15 s; A1 (judged static: small text typed from 0.8 s) 0.8 to 0.9 s;
 * A1r2, A1r3 and A1r7 (drift, a cursor) none.
 */
export const TYPE_INK = 0.0008;
export const TYPE_BOX = 0.05;
export const TYPE_KEEP = 0.75;
export const OPEN_TYPE_S = 0.6;
/**
 * D306: a straight cut between two footage shots isn't a one-take defect (the user: cuts between the
 * drone shots looked cleaner than shape transitions). Read from the picture, because footage drawn
 * by a graphic (biliup.frame, D290: A1-noskill is one graphic) leaves no trace on the timeline: of
 * the pixels the cut changes (CUT_CHANGED grey levels or more on the scan's grey), FOOTAGE_SHARE or
 * more are coloured (FOOTAGE_CHROMA or more between the R, G and B channels) on each side. Biliup's UI
 * is grey panels with grey text, so a UI cut changes colourless pixels on at least one side; texture
 * doesn't tell them apart (a panel's text is as busy as the sea at 64 px). Measured on the 30 films'
 * 62 cuts, the lower side's coloured share: footage to footage 0.46 to 0.91 (full-bleed or in the
 * editor's preview; A1-noskill's two drone swaps 0.64 and 0.60), UI 0 to 0.43 (A1-noskill4's timeline
 * over blurred footage 0.42); five dusk cuts in plain Claude's B1 and B1b (dark sea, grey concrete)
 * at 0.33 to 0.43 stay cuts, the safe side.
 */
export const CUT_CHANGED = 24;
export const FOOTAGE_CHROMA = 16;
export const FOOTAGE_SHARE = 0.45;

export interface MotionAnchor {
  readonly frame: number;
  readonly label: string;
}
export interface MotionStrip {
  readonly label: string;
  readonly at: number;
  readonly frames: ReadonlyArray<number>;
}

const clampTo = (duration: number) => (f: number) => Math.max(0, Math.min(duration - 1, Math.round(f)));

/**
 * The moments the strips and the scan look around: each storyboard beat's start (its scene change),
 * else one every 2 s; never more than `max`, picked evenly, frame 0 first.
 */
export function motionAnchors(beats: ReadonlyArray<StoryBeat> | undefined, fps: number, duration: number, max = MAX_STRIPS): Array<MotionAnchor> {
  const clamp = clampTo(duration);
  let all: Array<MotionAnchor> = (beats ?? []).map((b, i) => ({ frame: clamp(b.start * fps), label: `beat ${i + 1}${b.title ? ` ${b.title}` : ""}` }));
  const drop = dropBeat(beats);
  const dropFrame = drop === undefined ? undefined : clamp(beats![drop]!.start * fps);
  if (!all.length) {
    const every = Math.max(1, Math.round(2 * fps));
    for (let f = 0; f < duration; f += every) all.push({ frame: f, label: `${(f / fps).toFixed(1)} s` });
  }
  const seen = new Set<number>();
  all = all.filter((a) => (seen.has(a.frame) ? false : (seen.add(a.frame), true))).sort((a, b) => a.frame - b.frame);
  if (all.length <= max) return all;
  const picked = Array.from({ length: max }, (_, i) => all[Math.round((i * (all.length - 1)) / (max - 1))]!);
  // The drop always gets its strip (rehearsal 3: 6 of 12 beats picked evenly skipped it): it takes
  // the place of the nearest picked beat that isn't frame 0 or the last.
  if (dropFrame !== undefined && !picked.some((a) => a.frame === dropFrame)) {
    const inner = picked.map((a, i) => ({ i, d: Math.abs(a.frame - dropFrame) })).filter((x) => x.i > 0 && x.i < picked.length - 1).sort((a, b) => a.d - b.d)[0];
    if (inner) picked[inner.i] = all.find((a) => a.frame === dropFrame)!;
    picked.sort((a, b) => a.frame - b.frame);
  }
  return picked;
}

/** A storyboard beat as the motion facts read it. */
export interface StoryBeat {
  readonly start: number;
  readonly end?: number;
  readonly title?: string;
  readonly entry?: string;
  readonly sound?: string;
}

/** The storyboard beat that lands on the drop: the first whose title or sound names it, or none. */
export function dropBeat(beats: ReadonlyArray<StoryBeat> | undefined): number | undefined {
  const i = (beats ?? []).findIndex((b) => /\bdrop\b/i.test(`${b.title ?? ""}\n${b.sound ?? ""}`));
  return i < 0 ? undefined : i;
}

/**
 * What happens around the drop, from the scan's consecutive frames there (SCAN_WINDOW each side): how
 * much the picture changes over the frames before the drop frame and over those from it. A reveal
 * that lands on the drop changes most from it; one that already happened changes little there.
 */
export function dropLine(frames: ReadonlyArray<{ readonly frame: number; readonly gray: Uint8Array }>, drop: number, fps: number): string | undefined {
  const at = new Map(frames.map((x) => [x.frame, x.gray]));
  const a = at.get(drop - SCAN_WINDOW), b = at.get(drop - 1), c = at.get(drop + SCAN_WINDOW);
  if (!a || !b || !c) return undefined;
  const before = frameDiff(a, b), from = frameDiff(b, c);
  const verdict = from >= 2 * before && from >= POP_MIN ? "the big change lands on it" : before >= 2 * from && before >= POP_MIN ? "the big change came before it, the drop itself changes little" : from < POP_MIN && before < POP_MIN ? "little changes on either side of it" : "the change is spread across it";
  return `Around the drop (f ${drop}, ${secs(drop, fps)}): the picture changes ${(before * 100).toFixed(1)} % over the ${SCAN_WINDOW} frames before it and ${(from * 100).toFixed(1)} % over the ${SCAN_WINDOW} frames from it: ${verdict}. Look at its strip: the preview should sit inset until the drop and flood past the corners on it.`;
}

/** A strip per anchor: STRIP_FRAMES frames 1/12 s apart, two before the anchor and the rest after (clamped, distinct). */
export function stripPlan(anchors: ReadonlyArray<MotionAnchor>, fps: number, duration: number, count = STRIP_FRAMES, step = STRIP_STEP_S): Array<MotionStrip> {
  const clamp = clampTo(duration);
  const gap = Math.max(1, Math.round(step * fps));
  const before = Math.floor((count - 1) / 2.5);
  return anchors.map((a) => {
    let start = a.frame - before * gap;
    if (start + (count - 1) * gap > duration - 1) start = duration - 1 - (count - 1) * gap;
    const frames = [...new Set(Array.from({ length: count }, (_, i) => clamp(Math.max(0, start) + i * gap)))];
    return { label: a.label, at: a.frame, frames };
  });
}

/**
 * The scan's frames: every frame when the film fits the budget; else SCAN_WINDOW consecutive frames
 * on each side of every anchor, frame 0 and the last frame, and an even sampling (at most every
 * 0.15 s) with what's left of the budget.
 */
export function scanPlan(anchors: ReadonlyArray<MotionAnchor>, fps: number, duration: number, budget = SCAN_BUDGET): Array<number> {
  if (duration <= 0) return [];
  if (duration <= budget) return Array.from({ length: duration }, (_, i) => i);
  const clamp = clampTo(duration);
  const set = new Set<number>([0, 1, duration - 2, duration - 1].map(clamp));
  for (const a of anchors) for (let d = -SCAN_WINDOW; d <= SCAN_WINDOW; d++) set.add(clamp(a.frame + d));
  const left = Math.max(8, budget - set.size);
  // At most every 0.15 s (rehearsal 4: a 0.27 s empty stretch fits between samples 0.25 s apart).
  const every = Math.max(1, Math.round(0.15 * fps), Math.ceil(duration / left));
  for (let f = 0; f < duration; f += every) set.add(f);
  return [...set].sort((a, b) => a - b).slice(0, budget + 8);
}

/**
 * The strips' and the scan's frames, each once, in render order: first the strips' frames, the
 * scan's frames around each scene change (SCAN_WINDOW each side) and the first and last two
 * frames; then the rest of the scan. Each part sorted.
 */
export function motionRenderOrder(strips: ReadonlyArray<MotionStrip>, scanList: ReadonlyArray<number>, anchors: ReadonlyArray<MotionAnchor>, duration: number): Array<number> {
  const first = new Set<number>(strips.flatMap((s) => s.frames));
  for (const f of scanList) if (f <= 1 || f >= duration - 2 || anchors.some((a) => Math.abs(f - a.frame) <= SCAN_WINDOW)) first.add(f);
  const rest = new Set(scanList.filter((f) => !first.has(f)));
  return [...[...first].sort((a, b) => a - b), ...[...rest].sort((a, b) => a - b)];
}

export interface MotionFrames {
  /** Every frame rendered (the strips' and the scan's), by frame. */
  readonly byFrame: ReadonlyMap<number, Uint8Array>;
  /** The scan's frames that were rendered, in order. */
  readonly scanned: ReadonlyArray<number>;
  /** How many the scan planned (scanList). */
  readonly planned: number;
  /** Whether the time budget left some out. */
  readonly cut: boolean;
}

/**
 * The strips' and the scan's frames rendered once (motionRenderOrder) by `render`, which asks
 * `until` before each frame: once `budgetMs` has gone by (on `now`), the rest are left out.
 */
export async function motionFrames(o: {
  readonly strips: ReadonlyArray<MotionStrip>;
  readonly scanList: ReadonlyArray<number>;
  readonly anchors: ReadonlyArray<MotionAnchor>;
  readonly duration: number;
  readonly render: (frames: ReadonlyArray<number>, until: () => boolean) => Promise<ReadonlyArray<{ readonly frame: number; readonly jpeg: Uint8Array }>>;
  readonly budgetMs?: number;
  readonly now?: () => number;
}): Promise<MotionFrames> {
  const now = o.now ?? Date.now;
  const deadline = now() + (o.budgetMs ?? MOTION_BUDGET_MS);
  const order = motionRenderOrder(o.strips, o.scanList, o.anchors, o.duration);
  const rendered = await o.render(order, () => now() > deadline);
  const byFrame = new Map(rendered.map((f) => [f.frame, f.jpeg]));
  const scanned = [...new Set(o.scanList)].filter((f) => byFrame.has(f)).sort((a, b) => a - b);
  return { byFrame, scanned, planned: new Set(o.scanList).size, cut: rendered.length < order.length };
}

/** The most sampled steps the refinement narrows down, and its own time budget (ms). */
export const REFINE_MAX = 16;
export const REFINE_BUDGET_MS = 20_000;

/**
 * A1 (rehearsal 4): the scan judged jumps only where it had consecutive frames, the windows around
 * the storyboard's beats, so three hard cuts mid-beat (13, 14, 15 s) and a black-to-footage jump
 * (8.5 s) were never seen. This narrows each big step between two sampled frames down to the two
 * consecutive frames it happens between (bisection: the half that changes more, each round's
 * midpoints rendered together), then renders two frames on each side of it, so motionScan can judge
 * it as a jump or as motion. Picks the REFINE_MAX biggest sampled steps over POP_MIN, at most 0.5 s
 * apart; stops when `until` says so. Returns the grey frames it added.
 */
export async function refineJumps(o: {
  readonly frames: ReadonlyArray<ScanFrame>;
  readonly fps: number;
  readonly duration: number;
  readonly render: (frames: ReadonlyArray<number>) => Promise<ReadonlyArray<ScanFrame>>;
  readonly until?: () => boolean;
  readonly max?: number;
}): Promise<Array<ScanFrame>> {
  const at = new Map(o.frames.map((x) => [x.frame, x.gray]));
  const added: Array<ScanFrame> = [];
  const sorted = [...at.keys()].sort((a, b) => a - b);
  const gapMax = Math.max(2, Math.round(0.5 * o.fps));
  const steps: Array<{ lo: number; hi: number; d: number }> = [];
  for (let i = 1; i < sorted.length; i++) {
    const lo = sorted[i - 1]!, hi = sorted[i]!;
    if (hi - lo < 2 || hi - lo > gapMax) continue;
    const d = frameDiff(at.get(lo)!, at.get(hi)!);
    if (d >= POP_MIN) steps.push({ lo, hi, d });
  }
  let live = steps.sort((a, b) => b.d - a.d).slice(0, o.max ?? REFINE_MAX);
  const fetch = async (frames: Iterable<number>) => {
    const want = [...new Set(frames)].filter((f) => f >= 0 && f < o.duration && !at.has(f)).sort((a, b) => a - b);
    if (!want.length || o.until?.()) return !want.length;
    for (const x of await o.render(want)) if (!at.has(x.frame)) (at.set(x.frame, x.gray), added.push(x));
    return true;
  };
  // Bisect every live step together, a round of midpoints at a time.
  while (live.some((s) => s.hi - s.lo > 1)) {
    const mids = live.filter((s) => s.hi - s.lo > 1).map((s) => ({ s, m: (s.lo + s.hi) >> 1 }));
    if (!(await fetch(mids.map((x) => x.m)))) return added;
    // A midpoint that wasn't rendered drops its step.
    live = live.filter((s) => s.hi - s.lo <= 1 || at.has((s.lo + s.hi) >> 1));
    for (const { s, m } of mids) {
      const a = at.get(m);
      if (!a) continue;
      if (frameDiff(at.get(s.lo)!, a) >= frameDiff(a, at.get(s.hi)!)) s.hi = m;
      else s.lo = m;
    }
  }
  // Two frames each side of each narrowed step, for motionScan's neighbours.
  if (!(await fetch(live.flatMap((s) => [s.lo - 2, s.lo - 1, s.hi + 1, s.hi + 2])))) return added;
  // Round 4's A1: a quick swap spreads over a few frames; widen each side while its step is still a
  // quarter of the narrowed one, up to BURST_S plus the two still steps motionScan needs there.
  const reach = Math.max(2, Math.floor(BURST_S * o.fps)) + 2;
  const d = (x: number) => (at.has(x) && at.has(x + 1) ? frameDiff(at.get(x)!, at.get(x + 1)!) : 0);
  const wide: Array<number> = [];
  for (const s of live) {
    const q = d(s.lo) / 4;
    // A side still moving gets the rest of its reach in the same render.
    if (Math.max(d(s.lo - 2), d(s.lo - 1)) >= q) for (let x = s.lo - 3; x >= s.lo - reach; x--) wide.push(x);
    if (Math.max(d(s.hi), d(s.hi + 1)) >= q) for (let x = s.hi + 3; x <= s.hi + reach; x++) wide.push(x);
  }
  if (wide.length) await fetch(wide);
  return added;
}

/** A scan frame: its grey at the scan's size, and (optional) its emptiness measured at EMPTY_WIDTH and its information at THIN_WIDTH. */
export interface ScanFrame {
  readonly frame: number;
  readonly gray: Uint8Array;
  readonly bare?: EmptyMeasure;
  readonly info?: ThinMeasure;
  /** D306: each pixel's colour at the scan's size (chromaOf: max minus min of R, G, B), for footage cuts. */
  readonly chroma?: Uint8Array;
  /** D306: the grey at thinSize (motionScan's `fine` size), for typing. */
  readonly fine?: Uint8Array;
}

/** Each pixel's colour (max minus min of its R, G and B) from packed RGB (3 bytes a pixel). */
export function chromaOf(rgb: Uint8Array): Uint8Array {
  const out = new Uint8Array(Math.floor(rgb.length / 3));
  for (let i = 0; i < out.length; i++) {
    const r = rgb[3 * i]!, g = rgb[3 * i + 1]!, b = rgb[3 * i + 2]!;
    out[i] = Math.max(r, g, b) - Math.min(r, g, b);
  }
  return out;
}

/**
 * D306: whether a cut from grey a to grey b (with each pixel's colour) is between two footage shots:
 * FOOTAGE_SHARE or more of the pixels it changes are coloured on each side (`photo`: the lower
 * side's share). Undefined when it changes too little to tell (under 2 % of the picture).
 */
export function footageCut(a: Uint8Array, b: Uint8Array, ca: Uint8Array, cb: Uint8Array): { readonly footage: boolean; readonly photo: number } | undefined {
  const n = Math.min(a.length, b.length, ca.length, cb.length);
  let changed = 0, pa = 0, pb = 0;
  for (let i = 0; i < n; i++) {
    if (Math.abs(a[i]! - b[i]!) < CUT_CHANGED) continue;
    changed++;
    if (ca[i]! >= FOOTAGE_CHROMA) pa++;
    if (cb[i]! >= FOOTAGE_CHROMA) pb++;
  }
  if (!n || changed < 0.02 * n) return undefined;
  const photo = round(Math.min(pa, pb) / changed, 2);
  return { footage: photo >= FOOTAGE_SHARE, photo };
}

/** D306: the strong changes (THIN_CONTRAST or more) between two fine greys (w wide): which pixels, and their box's share of the frame. */
function inkChange(a: Uint8Array, b: Uint8Array, w: number): { readonly at: Array<number>; readonly box: number } {
  const n = Math.min(a.length, b.length);
  const h = Math.floor(n / w);
  const at: Array<number> = [];
  let x0 = w, x1 = -1, y0 = h, y1 = -1;
  for (let i = 0; i < n; i++) {
    if (Math.abs(a[i]! - b[i]!) < THIN_CONTRAST) continue;
    at.push(i);
    const x = i % w, y = (i / w) | 0;
    if (x < x0) x0 = x;
    if (x > x1) x1 = x;
    if (y < y0) y0 = y;
    if (y > y1) y1 = y;
  }
  return { at, box: at.length && n ? ((x1 - x0 + 1) * (y1 - y0 + 1)) / n : 0 };
}

/** Whether a grey frame (w × h) is nearly empty (emptyMeasure). */
export const nearlyEmpty = (gray: Uint8Array, w: number, h: number) => emptyMeasure(gray, w, h).empty;

/** The emptiness budget's own slice (ms), after refineJumps' REFINE_BUDGET_MS. */
export const REFINE_EMPTY_BUDGET_MS = 12_000;

/**
 * The nearly empty stretches the samples show or hint at, sampled closer and then edged exactly:
 * first every gap over EMPTY_STEP_S next to a bare sample (empty, or EMPTY_SUSPECT or barer) is
 * filled at EMPTY_STEP_S (a short stretch fits between samples 0.5 s apart); then between each empty
 * sample and its non-empty neighbour (at most 0.5 s apart), bisection down to two consecutive frames,
 * a round of midpoints rendered together. At most `max` edges; stops when `until` says so. A frame's
 * emptiness is its `bare`, else measured on its grey at `size`. The frames it added.
 */
export async function refineEmpty(o: {
  readonly frames: ReadonlyArray<ScanFrame>;
  readonly fps: number;
  readonly size: { readonly w: number; readonly h: number };
  readonly render: (frames: ReadonlyArray<number>) => Promise<ReadonlyArray<ScanFrame>>;
  readonly until?: () => boolean;
  readonly max?: number;
  readonly duration?: number;
}): Promise<Array<ScanFrame>> {
  const at = new Map(o.frames.map((x) => [x.frame, x]));
  const measured = new Map<number, EmptyMeasure>();
  const bareOf = (f: number) => {
    if (!measured.has(f)) {
      const x = at.get(f)!;
      measured.set(f, x.bare ?? emptyMeasure(x.gray, o.size.w, o.size.h));
    }
    return measured.get(f)!;
  };
  // A low-information frame (info.thin) is sampled and edged like an empty one.
  const isBlank = (f: number) => bareOf(f).empty || !!at.get(f)!.info?.thin;
  const added: Array<ScanFrame> = [];
  const take = async (frames: ReadonlyArray<number>) => {
    const want = [...new Set(frames)].filter((f) => f >= 0 && (o.duration === undefined || f < o.duration) && !at.has(f)).sort((a, b) => a - b);
    if (!want.length) return;
    for (const x of await o.render(want)) if (!at.has(x.frame)) (at.set(x.frame, x), added.push(x));
  };
  const sortedNow = () => [...at.keys()].sort((a, b) => a - b);
  // 1. Closer samples next to a bare one.
  const step = Math.max(1, Math.round(EMPTY_STEP_S * o.fps));
  const gapMax = Math.max(2, Math.round(0.5 * o.fps));
  {
    const sorted = sortedNow();
    const fill: Array<number> = [];
    for (let i = 1; i < sorted.length; i++) {
      const lo = sorted[i - 1]!, hi = sorted[i]!;
      if (hi - lo <= step || hi - lo > 4 * gapMax) continue;
      const suspect = (f: number) => isBlank(f) || bareOf(f).content <= EMPTY_SUSPECT;
      if (!suspect(lo) && !suspect(hi)) continue;
      const n = Math.ceil((hi - lo) / step);
      for (let k = 1; k < n; k++) fill.push(lo + Math.round(((hi - lo) * k) / n));
    }
    if (fill.length && !o.until?.()) await take(fill);
  }
  // 2. Each edge between an empty sample and a non-empty one, narrowed to two consecutive frames.
  const sorted = sortedNow();
  let live: Array<{ lo: number; hi: number }> = [];
  for (let i = 1; i < sorted.length; i++) {
    const lo = sorted[i - 1]!, hi = sorted[i]!;
    if (hi - lo >= 2 && hi - lo <= gapMax && isBlank(lo) !== isBlank(hi)) live.push({ lo, hi });
  }
  live = live.slice(0, o.max ?? 64);
  while (live.some((e) => e.hi - e.lo > 1)) {
    const mids = live.filter((e) => e.hi - e.lo > 1).map((e) => ({ e, m: (e.lo + e.hi) >> 1 }));
    if (o.until?.()) break;
    await take(mids.map((x) => x.m));
    live = live.filter((e) => e.hi - e.lo <= 1 || at.has((e.lo + e.hi) >> 1));
    for (const { e, m } of mids) {
      if (!at.has(m)) continue;
      if (isBlank(m) === isBlank(e.lo)) e.lo = m;
      else e.hi = m;
    }
  }
  return added;
}

/** Mean absolute difference of two grey frames, 0..1. */
export function frameDiff(a: Uint8Array, b: Uint8Array): number {
  const n = Math.min(a.length, b.length);
  if (!n) return 0;
  let s = 0;
  for (let i = 0; i < n; i++) s += Math.abs(a[i]! - b[i]!);
  return s / n / 255;
}

const median = (xs: ReadonlyArray<number>) => {
  if (!xs.length) return 0;
  const s = [...xs].sort((a, b) => a - b);
  const m = s.length >> 1;
  return s.length % 2 ? s[m]! : (s[m - 1]! + s[m]!) / 2;
};

export interface MotionEvent {
  readonly frame: number;
  readonly diff: number;
  /** How many times its neighbours' difference. */
  readonly ratio: number;
  /** A quick swap: the frames the change took (2 or more); a one-frame jump has none. */
  readonly span?: number;
  /** D306 (jumps): a cut between two footage shots (footageCut), and its photographic share, when the frames carry colour. */
  readonly footage?: boolean;
  readonly photo?: number;
}
export interface MotionScan {
  readonly frames: number;
  /** Pops: a frame unlike both neighbours, which agree; a flash (span: 2 to FLASH_S worth of frames) is a short run of them. */
  readonly pops: ReadonlyArray<MotionEvent>;
  /** One-frame jumps: the picture changes a lot between two frames and little around them (a cut, a one-frame flood). */
  readonly jumps: ReadonlyArray<MotionEvent>;
  /** Stretches (frames) where nothing visible changes, over HOLD_MAX_S. */
  readonly holds: ReadonlyArray<{ readonly start: number; readonly end: number }>;
  /**
   * Runs (frames, inclusive) where the frame is nearly empty (emptyMeasure), from the first empty
   * sample to the last, when the scan could measure it. `upto`: when an edge wasn't narrowed to the
   * frame, how far the run may reach (out to the neighbouring non-empty samples, exclusive).
   */
  readonly empty: ReadonlyArray<EmptyRun>;
  readonly emptyMeasured: boolean;
  /** Runs of low-information frames (thinMeasure), built like `empty`, when the frames carry it. */
  readonly thin?: ReadonlyArray<EmptyRun>;
  readonly thinMeasured?: boolean;
  /** Fast moves (frames, the steps' first and last frame) and their biggest step. */
  readonly fast: ReadonlyArray<{ readonly start: number; readonly end: number; readonly peak: number }>;
  /**
   * The open (when the samples reach OPEN_MAX_S): the first sample by which OPEN_MOVE of the picture
   * changed over about OPEN_WINDOW_S, if any within the first 2 s, and the biggest such change before OPEN_MAX_S.
   */
  readonly open?: { readonly frame?: number; readonly peak: number; readonly typing?: number };
  /** The loop seam: the last frame against frame 0, and the typical frame-to-frame difference. */
  readonly seam?: { readonly diff: number; readonly typical: number };
  readonly typical: number;
}

/**
 * Pops, jumps, holds and the seam in a set of grey frames (any frames; consecutive ones are what
 * pops and jumps are judged on). Deterministic and pure.
 */
export function motionScan(input: ReadonlyArray<ScanFrame>, fps: number, o: { readonly ratio?: number; readonly min?: number; readonly size?: { readonly w: number; readonly h: number }; readonly fine?: { readonly w: number; readonly h: number } } = {}): MotionScan {
  const ratio = o.ratio ?? POP_RATIO;
  const min = o.min ?? POP_MIN;
  const fr = [...input].sort((a, b) => a.frame - b.frame);
  const at = new Map(fr.map((x) => [x.frame, x.gray]));
  const bareAt = new Map(fr.map((x) => [x.frame, x.bare]));
  // D306: whether a cut from frame x to frame y is between footage shots (both carry colour).
  const chromaAt = new Map(fr.flatMap((x) => (x.chroma ? [[x.frame, x.chroma] as const] : [])));
  const footageOf = (x: number, y: number): { footage?: boolean; photo?: number } => {
    const ca = chromaAt.get(x), cb = chromaAt.get(y);
    const cut = ca && cb ? footageCut(at.get(x)!, at.get(y)!, ca, cb) : undefined;
    return cut ? { footage: cut.footage, photo: cut.photo } : {};
  };
  // Differences between consecutive frames (frame f → f + 1).
  const step = new Map<number, number>();
  for (const x of fr) {
    const next = at.get(x.frame + 1);
    if (next) step.set(x.frame, frameDiff(x.gray, next));
  }
  const typical = Math.max(median([...step.values()]), 0.0005);
  const pops: Array<MotionEvent> = [];
  const popFrames = new Set<number>();
  for (const x of fr) {
    const a = step.get(x.frame - 1);
    const b = step.get(x.frame);
    if (a === undefined || b === undefined) continue;
    const around = frameDiff(at.get(x.frame - 1)!, at.get(x.frame + 1)!);
    const d = Math.min(a, b);
    const base = Math.max(around, typical);
    if (d >= min && d >= ratio * base) {
      pops.push({ frame: x.frame, diff: round(d), ratio: round(d / base, 1) });
      popFrames.add(x.frame);
    }
  }
  // Flashes (FLASH_S): frames f..f + n - 1 unlike f - 1 and f + n, which agree.
  const flashMax = Math.max(2, Math.floor(FLASH_S * fps));
  for (const x of fr) {
    const f = x.frame;
    if (popFrames.has(f)) continue;
    const enter = step.get(f - 1);
    if (enter === undefined || enter < min) continue;
    for (let n = 2; n <= flashMax; n++) {
      const last = f + n - 1;
      const leave = step.get(last);
      const after = at.get(last + 1);
      if (leave === undefined || !after) break;
      let inner = 0;
      for (let k = f; k < last; k++) inner = Math.max(inner, step.get(k) ?? Infinity);
      if (!Number.isFinite(inner)) break;
      const d = Math.min(enter, leave);
      const base = Math.max(frameDiff(at.get(f - 1)!, after), inner, typical);
      if (d >= min && d >= ratio * base && ![...Array(n).keys()].some((k) => popFrames.has(f + k))) {
        pops.push({ frame: f, diff: round(d), ratio: round(d / base, 1), span: n });
        for (let k = 0; k < n; k++) popFrames.add(f + k);
        break;
      }
    }
  }
  pops.sort((a, b) => a.frame - b.frame);
  const jumps: Array<MotionEvent> = [];
  for (const [f, d] of step) {
    if (popFrames.has(f) || popFrames.has(f + 1) || d < min) continue;
    // A cut is judged against the calmer side: the shot it cuts into may move fast from its first
    // frame (A1 13 s: a 19 % cut into a push-in easing out from 7 %/frame), so the quieter side sets
    // the bar (POP_RATIO), and the busier one only needs to be clearly smaller (JUMP_OVER), which a
    // smooth move never is (an ease changes a few % frame to frame).
    const side = (xs: ReadonlyArray<number | undefined>) => {
      const ys = xs.filter((v): v is number => v !== undefined);
      return ys.length ? Math.max(...ys) : undefined;
    };
    const before = side([step.get(f - 2), step.get(f - 1)]);
    const after = side([step.get(f + 1), step.get(f + 2)]);
    if (before === undefined && after === undefined) continue;
    const calm = Math.max(Math.min(before ?? Infinity, after ?? Infinity), typical);
    const busy = Math.max(before ?? 0, after ?? 0);
    if (d >= ratio * calm && d >= JUMP_OVER * busy) jumps.push({ frame: f + 1, diff: round(d), ratio: round(d / calm, 1), ...footageOf(f, f + 1) });
  }
  // Quick swaps (BURST_S): a jump spread over a few frames, every step of it and around it known.
  const burstMax = Math.max(2, Math.floor(BURST_S * fps));
  const burstSteps = new Set<number>();
  for (const [f, peak] of step) {
    if (peak < Math.max(min, BURST_PEAK) || !((step.get(f - 1) ?? -1) <= peak && (step.get(f + 1) ?? Infinity) < peak)) continue;
    const q = peak / 4;
    let a = f, b = f;
    while ((step.get(a - 1) ?? -1) >= q && f - a < burstMax) a--;
    while ((step.get(b + 1) ?? -1) >= q && b - f < burstMax) b++;
    const len = b - a + 1;
    const out = [a - 1, a - 2, b + 1, b + 2].map((x) => step.get(x));
    if (len < 2 || len > burstMax || out.some((v) => v === undefined || v >= q)) continue;
    if (jumps.some((j) => j.frame - 1 >= a && j.frame - 1 <= b) || [...popFrames].some((x) => x >= a && x <= b + 1)) continue;
    // A close into a nearly empty canvas is an exit, not a cut from one picture to another: the
    // reference 2.mp4's iris at 26.5 s shrinks the shot to a dot in 4 frames (content 0.05 % two
    // frames after). The empty stretch after it is the empty rules' to judge.
    if (bareAt.get(b + 1)?.empty || bareAt.get(b + 2)?.empty) continue;
    // A swap goes from one picture to another (every pixel one way: the net change is most of the
    // steps' sum); a whip of a pattern moves it back and forth (a fast move, not a cut).
    const net = frameDiff(at.get(a)!, at.get(b + 1)!);
    let sum = 0;
    for (let x = a; x <= b; x++) sum += step.get(x)!;
    if (net < BURST_NET * sum) continue;
    jumps.push({ frame: f + 1, diff: round(net), ratio: round(peak / Math.max(...(out as Array<number>), typical), 1), span: len, ...footageOf(a, b + 1) });
    for (let x = a; x <= b; x++) burstSteps.add(x);
  }
  jumps.sort((a, b) => a.frame - b.frame);
  // Holds: runs of samples (at most 0.5 s apart) that don't change.
  const holds: Array<{ start: number; end: number }> = [];
  let runStart: number | undefined;
  let runEnd = 0;
  const gapMax = Math.max(1, Math.round(0.5 * fps));
  const close = () => {
    if (runStart !== undefined && (runEnd - runStart) / fps > HOLD_MAX_S) holds.push({ start: runStart, end: runEnd });
    runStart = undefined;
  };
  // D306: a hold ends where the fine grey has changed TYPE_INK or more since the run began (typing
  // under 1 % of the 64 px grey); a blinking cursor never adds up to that.
  const fineW = o.fine?.w;
  let anchor: Uint8Array | undefined;
  const typed = (c: ScanFrame) => !!(fineW && anchor && c.fine && inkChange(anchor, c.fine, fineW).at.length >= TYPE_INK * c.fine.length);
  for (let i = 1; i < fr.length; i++) {
    const p = fr[i - 1]!;
    const c = fr[i]!;
    if (runStart === undefined) anchor = p.fine;
    if (c.frame - p.frame <= gapMax && frameDiff(p.gray, c.gray) < HOLD_DIFF && !typed(c)) {
      if (runStart === undefined) runStart = p.frame;
      runEnd = c.frame;
    } else close();
  }
  close();
  // Nearly empty runs: consecutive samples (at most 0.5 s apart) that are nearly empty; a run is
  // measured from its first empty sample to its last (refineEmpty renders its exact edges), and an
  // edge that isn't exact reaches out to the neighbouring sample (upto).
  const runsOf = (pick: ReadonlyArray<boolean>, of: ReadonlyArray<ScanFrame>): Array<EmptyRun> => {
    const out: Array<EmptyRun> = [];
    const runs: Array<{ start: number; end: number; first: number; last: number }> = [];
    let run: { start: number; end: number; first: number; last: number } | undefined;
    of.forEach((x, i) => {
      if (pick[i] && run && x.frame - run.end <= gapMax) (run.end = x.frame), (run.last = i);
      else {
        if (run) runs.push(run);
        run = pick[i] ? { start: x.frame, end: x.frame, first: i, last: i } : undefined;
      }
    });
    if (run) runs.push(run);
    for (const r of runs) {
      const before = of[r.first - 1]?.frame, after = of[r.last + 1]?.frame;
      const lo = before === undefined ? r.start : before + 1, hi = after === undefined ? r.end : after - 1;
      out.push(lo < r.start || hi > r.end ? { start: r.start, end: r.end, upto: { start: lo, end: hi } } : { start: r.start, end: r.end });
    }
    return out;
  };
  const emptyMeasured = !!o.size || fr.some((x) => x.bare);
  const empty = emptyMeasured ? runsOf(fr.map((x) => x.bare?.empty ?? (o.size ? emptyMeasure(x.gray, o.size.w, o.size.h).empty : false)), fr) : [];
  // Low-information runs, over the frames that carry the measure (the scan's grey is too small for it).
  const informed = fr.filter((x) => x.info);
  const thinMeasured = informed.length > 0;
  const thin = thinMeasured ? runsOf(informed.map((x) => x.info!.thin), informed) : [];
  // Fast moves: FAST_RUN or more consecutive steps of FAST_STEP or more, none of them a jump or a pop.
  const fast: Array<{ start: number; end: number; peak: number }> = [];
  const skip = new Set([...jumps.map((j) => j.frame - 1), ...burstSteps, ...popFrames, ...[...popFrames].map((f) => f - 1)]);
  let fs: { start: number; end: number; peak: number; n: number } | undefined;
  for (const x of fr) {
    const d = step.get(x.frame);
    const quick = d !== undefined && d >= FAST_STEP && !skip.has(x.frame);
    if (quick && fs && fs.end === x.frame) {
      fs.end = x.frame + 1;
      fs.n++;
      fs.peak = Math.max(fs.peak, d);
    } else {
      if (fs && fs.n >= FAST_RUN) fast.push({ start: fs.start, end: fs.end, peak: round(fs.peak) });
      fs = quick ? { start: x.frame, end: x.frame + 1, peak: d, n: 1 } : undefined;
    }
  }
  if (fs && fs.n >= FAST_RUN) fast.push({ start: fs.start, end: fs.end, peak: round(fs.peak) });
  const open = openMove(fr, fps, fineW);
  const first = fr[0];
  const last = fr.at(-1);
  const seam = first && last && last.frame > first.frame ? { diff: round(frameDiff(last.gray, first.gray)), typical: round(typical) } : undefined;
  return { frames: fr.length, pops, jumps, holds, empty, emptyMeasured, thin, thinMeasured, fast, ...(open ? { open } : {}), ...(seam ? { seam } : {}), typical: round(typical) };
}

/**
 * The open's first real change (MotionScan.open): each sample in the first 2 s against the latest
 * sample at least OPEN_WINDOW_S before it (at most twice that), so every frame or a sampling every
 * 0.15 s measure the same window. Undefined when the samples start late or stop before OPEN_MAX_S.
 */
function openMove(fr: ReadonlyArray<ScanFrame>, fps: number, fineW?: number): MotionScan["open"] {
  const win = Math.max(1, Math.round(OPEN_WINDOW_S * fps));
  const until = Math.round(OPEN_MAX_S * fps);
  const head = fr.filter((x) => x.frame <= 2 * fps);
  if (head[0]?.frame !== 0 || head.at(-1)!.frame < until) return undefined;
  let frame: number | undefined;
  let peak = 0;
  for (let i = 1, j = 0; i < head.length; i++) {
    const b = head[i]!;
    while (j + 1 < i && head[j + 1]!.frame <= b.frame - win) j++;
    const a = head[j]!;
    if (b.frame - a.frame < win || b.frame - a.frame > 2 * win) continue;
    const d = frameDiff(a.gray, b.gray);
    if (b.frame <= until) peak = Math.max(peak, d);
    if (frame === undefined && d >= OPEN_MOVE) frame = b.frame;
  }
  const typing = fineW ? typingStart(head, win, fineW, Math.round(OPEN_TYPE_S * fps)) : undefined;
  return { ...(frame !== undefined ? { frame } : {}), peak: round(peak), ...(typing !== undefined ? { typing } : {}) };
}

/**
 * D306: the first sample (by `until`) where typing shows on the fine grey: against a sample one to
 * two windows before it, TYPE_INK or more of the picture changes strongly inside a box of TYPE_BOX
 * or less, and TYPE_KEEP of those pixels still differ from that sample one to three windows later.
 */
function typingStart(head: ReadonlyArray<ScanFrame>, win: number, w: number, until: number): number | undefined {
  for (const b of head) {
    if (b.frame > until) break;
    if (!b.fine) continue;
    const a = head.findLast((x) => x.fine && x.frame <= b.frame - win && x.frame >= b.frame - 2 * win);
    const c = head.find((x) => x.fine && x.frame >= b.frame + win && x.frame <= b.frame + 3 * win);
    if (!a || !c) continue;
    const ink = inkChange(a.fine!, b.fine, w);
    if (ink.at.length < TYPE_INK * b.fine.length || ink.box > TYPE_BOX) continue;
    const kept = ink.at.filter((i) => Math.abs(a.fine![i]! - c.fine![i]!) >= THIN_CONTRAST).length;
    if (kept >= TYPE_KEEP * ink.at.length) return b.frame;
  }
  return undefined;
}

/** Whether the open is static: its first real change comes after OPEN_MAX_S (or not in the first 2 s). */
export const openLate = (scan: MotionScan, fps: number) => !!scan.open && (scan.open.frame === undefined || scan.open.frame > OPEN_MAX_S * fps) && !(scan.open.typing !== undefined && scan.open.typing <= OPEN_TYPE_S * fps);

const round = (v: number, digits = 4) => Math.round(v * 10 ** digits) / 10 ** digits;
const secs = (f: number, fps: number) => `${(f / fps).toFixed(2)} s`;

/** Whether a seam is a stutter: the last frame further from frame 0 than POP_RATIO typical steps (and visibly so). */
export const seamBroken = (scan: MotionScan) => !!scan.seam && scan.seam.diff >= Math.max(POP_MIN / 2, POP_RATIO * scan.seam.typical);

/**
 * The scan as review findings (rule "motion"): pops always; one-frame jumps and holds over a second
 * when the film is one take (a cut is fine elsewhere, an end card may hold); the seam when it loops.
 * Nearly empty runs of EMPTY_MIN_S or more (blockers when hard from EMPTY_HARD_S), low-information runs (from THIN_HARD_S) and fast moves (never blockers) when one take.
 * At most 12, the blockers first, worst pops first. hard (a motion film, D297): pops, jumps and the seam are blockers.
 * D306: a cut between two footage shots (footageCut, or `clipCuts`: the timeline's own straight cuts
 * from one clip to the next) is never one, one take or not; `cutsHard` (default `hard`) says whether
 * the other cuts block.
 */
export function motionFindings(scan: MotionScan, fps: number, o: { readonly oneTake?: boolean; readonly loop?: boolean; readonly hard?: boolean; readonly still?: number; readonly clipCuts?: ReadonlyArray<number>; readonly cutsHard?: boolean } = {}): Array<CriticFinding> {
  // D297: in a motion film (hard) a pop, a one-frame jump (a cut) and a broken seam fail the round.
  const hard = !!o.hard;
  const out: Array<CriticFinding> = [];
  for (const p of [...scan.pops].sort((a, b) => b.ratio - a.ratio).slice(0, 4)) {
    out.push(p.span
      ? { rule: "motion", key: `motion:pop:${p.frame}`, frame: p.frame, range: { start: p.frame, end: p.frame + p.span - 1 }, hard, message: `A ${p.span}-frame flash at f ${p.frame} to f ${p.frame + p.span - 1} (${secs(p.frame, fps)}): those frames differ from the frames on both sides ${p.ratio}× as much as those differ from each other (mean change ${(p.diff * 100).toFixed(1)} %): something shows for ${p.span} frames and the picture goes back, like the next shot drawn whole before its transition starts. Check the frames' branches: the state the transition starts from must be drawn from its first frame on.` }
      : { rule: "motion", key: `motion:pop:${p.frame}`, frame: p.frame, hard, message: `Single-frame pop at f ${p.frame} (${secs(p.frame, fps)}): that frame differs from both neighbours ${p.ratio}× as much as they differ from each other (mean change ${(p.diff * 100).toFixed(1)} %). Something shows or vanishes for one frame.` });
  }
  if (o.oneTake) {
    const cutsHard = o.cutsHard ?? hard;
    for (const j of scan.jumps.filter((x) => !footageJump(x, o.clipCuts)).slice(0, 4)) out.push({ rule: "motion", key: `motion:jump:${j.frame}`, frame: j.frame, hard: cutsHard, message: j.span ? `Quick swap at f ${j.frame} (${secs(j.frame, fps)}): ${(j.diff * 100).toFixed(1)} % of the picture changes in ${j.span} frames (${(j.span / fps).toFixed(2)} s) with stillness on both sides, its fastest frame ${j.ratio}× the frames around it; to the eye that is a cut. In a one-take film carry the old state into the new one (a morph, a grow or a flood over about 0.3 s), not a quick fade or swap.` : `One-frame jump at f ${j.frame} (${secs(j.frame, fps)}): ${(j.diff * 100).toFixed(1)} % of the picture changes in one frame, ${j.ratio}× the frames around it; in a one-take film that is a cut or a flood that didn't overscale over about 0.3 s.` });
    // The longest holds (an end card held 5 s came third in time order and wasn't listed).
    const still = o.still;
    for (const h of [...scan.holds].sort((a, b) => b.end - b.start - (a.end - a.start)).slice(0, 2)) {
      const len = (h.end - h.start) / fps;
      out.push(still !== undefined && len >= still
        ? { rule: "motion", key: `motion:still:${h.start}`, frame: h.start, range: { start: h.start, end: h.end }, hard: false, message: `One still frame from f ${h.start} to f ${h.end} (${secs(h.start, fps)} to ${secs(h.end, fps)}, ${len.toFixed(1)} s): nothing on screen moves for over ${still} s. The films that worked hold their end card about 1.5 to 3 s; shorten the hold or keep one thing moving through it (the last word landing, a slow push on the lockup), unless the request asks for the hold.` }
        : { rule: "motion", key: `motion:hold:${h.start}`, frame: h.start, range: { start: h.start, end: h.end }, hard: false, message: `Nothing changes from f ${h.start} to f ${h.end} (${len.toFixed(1)} s): a hold over 1 s.` });
    }
  }
  // A motion film that isn't one take (cuts are fine) still shouldn't park on one frame.
  if (!o.oneTake && o.still !== undefined)
    for (const h of [...scan.holds].filter((x) => (x.end - x.start) / fps >= o.still!).sort((a, b) => b.end - b.start - (a.end - a.start)).slice(0, 2)) {
      const len = (h.end - h.start) / fps;
      out.push({ rule: "motion", key: `motion:still:${h.start}`, frame: h.start, range: { start: h.start, end: h.end }, hard: false, message: `One still frame from f ${h.start} to f ${h.end} (${secs(h.start, fps)} to ${secs(h.end, fps)}, ${len.toFixed(1)} s): nothing on screen moves for over ${o.still} s. The films that worked hold their end card about 1.5 to 3 s; shorten the hold or keep one thing moving through it (the last word landing, a slow push on the lockup), unless the request asks for the hold.` });
    }
  // Blind round 3: a motionless first 1.5 to 2 s under a hook headline (drift only) read as a soft hook.
  if (o.oneTake && openLate(scan, fps)) {
    const at = scan.open!.frame;
    out.push({ rule: "motion", key: "motion:open:0", frame: 0, range: { start: 0, end: at ?? Math.round(2 * fps) }, hard: false, message: `A static open: ${at === undefined ? "nothing really changes in the first 2 s" : `the first real change lands at f ${at} (${secs(at, fps)})`}; until ${OPEN_MAX_S} s at most ${(scan.open!.peak * 100).toFixed(1)} % of the picture changes in ${OPEN_WINDOW_S} s (a drift is not a change). Make frame 0 move: the headline folding into the hero, the cursor's first click or keystroke landing by about 0.5 s.` });
  }
  // Rehearsal 4: nearly empty frames (A1 22.1 s, A2 4.2 s and 24 s) survived three rounds as a critic opinion.
  for (const e of o.oneTake ? emptyRuns(scan, fps).slice(0, 2) : []) {
    // A blocker only when the empty samples alone span it; one that may reach it is a finding to look at.
    const len = runSeconds(e, fps);
    const sure = len >= EMPTY_MIN_S - 1e-9;
    const fix = "Every frame should show the hero: morph the outgoing shape straight into the next one instead of clearing the frame between them.";
    const what = `at most a cursor or a speck (${+(EMPTY_CONTENT * 100).toFixed(1)} % or less of it isn't bare canvas, in ${EMPTY_PARTS} marks or fewer)`;
    out.push(sure
      ? { rule: "motion", key: `motion:empty:${e.start}`, frame: e.start, range: { start: e.start, end: e.end }, hard: hard && len >= EMPTY_HARD_S - 1e-9, message: `Nearly empty frame from f ${e.start} to f ${e.end} (${secs(e.start, fps)} to ${secs(e.end + 1, fps)}, ${runSeconds(e, fps).toFixed(2)} s): ${what}. ${fix}` }
      : { rule: "motion", key: `motion:empty:${e.start}`, frame: e.start, range: { start: e.upto!.start, end: e.upto!.end }, hard: false, message: `Maybe a nearly empty frame: f ${e.start} to f ${e.end} (${secs(e.start, fps)}) is ${what}, and it may last until the next sampled frames that aren't, f ${e.upto!.start} to f ${e.upto!.end} (up to ${runSeconds(e.upto!, fps).toFixed(2)} s; the scan couldn't narrow its edges in time). render_frames around it to see how long. ${fix}` });
  }
  // T1, T1b: the agent panel after Send, one bubble and desktop-size steps in a big dark field, passed
  // the empty rule (the panel counts as content) and every round.
  for (const e of o.oneTake ? thinRuns(scan, fps).slice(0, 2) : []) {
    const len = runSeconds(e, fps);
    const sure = len >= EMPTY_MIN_S - 1e-9;
    const what = `low-information: ${Math.round(THIN_FLAT * 100)} % or more of it one flat colour (a dark panel on the dark canvas counts as one), and nothing in it taller than ${+(THIN_TALL * 100).toFixed(1)} % of the height (a line or two of desktop-size text, a cursor)`;
    const fix = "Give the moment one big element: the line it is about set large (an biliupUI piece over the blurred stage, or the camera framing that line with biliupUI.frameText until its words reach 3.5 % or more), never a near-empty panel waiting for its content.";
    out.push(sure
      ? { rule: "motion", key: `motion:thin:${e.start}`, frame: e.start, range: { start: e.start, end: e.end }, hard: hard && len >= THIN_HARD_S - 1e-9, message: `A low-information frame from f ${e.start} to f ${e.end} (${secs(e.start, fps)} to ${secs(e.end + 1, fps)}, ${len.toFixed(2)} s): ${what}. ${fix}` }
      : { rule: "motion", key: `motion:thin:${e.start}`, frame: e.start, range: { start: e.upto!.start, end: e.upto!.end }, hard: false, message: `Maybe a low-information frame: f ${e.start} to f ${e.end} (${secs(e.start, fps)}) is ${what}, and it may last until f ${e.upto!.start} to f ${e.upto!.end} (up to ${runSeconds(e.upto!, fps).toFixed(2)} s; the scan couldn't narrow its edges in time). ${fix}` });
  }
  for (const m of o.oneTake ? scan.fast.slice(0, 3) : []) {
    out.push({ rule: "motion", key: `motion:fast:${m.start}`, frame: m.start, range: { start: m.start, end: m.end }, hard: false, message: `Very fast move from f ${m.start} to f ${m.end} (${secs(m.start, fps)}): up to ${(m.peak * 100).toFixed(0)} % of the picture changes every frame (listed from ${Math.round(FAST_STEP * 100)} %). The export's motion blur smears each frame over how far things move during its shutter; sub-frames only decide whether that shows as stacked copies (few) or one smooth smear (many), so more sub-frames only smooth the copies into a smear (render_frames draws no blur; the critic sees it drawn with the project's blur). If text smears or doubles there, slow the move's fastest frames (the ease's peak speed counts, not its length: a gentler curve or a shorter travel), keep the words still while it moves, or shorten the shutter. A smooth blur that keeps the words readable is fine as it is.` });
  }
  if (o.loop && seamBroken(scan)) out.push({ rule: "motion", key: "motion:seam:0", frame: 0, hard, message: `The loop seam: the last frame differs from frame 0 by ${(scan.seam!.diff * 100).toFixed(1)} % (a typical frame step is ${(scan.seam!.typical * 100).toFixed(2)} %), so the loop jumps. Make the last frame equal the first, cursor position and speed included.` });
  // The blockers first, so the cap never drops one (the seam came last) for a soft finding.
  return [...out.filter((f) => f.hard), ...out.filter((f) => !f.hard)].slice(0, 12);
}

/**
 * D306: whether a jump is a cut between footage shots: the picture says so (footageCut), or it lands
 * (within a frame, across its span) on one of the timeline's straight cuts from a clip to a clip.
 */
export function footageJump(j: MotionEvent, clipCuts: ReadonlyArray<number> = []): boolean {
  if (j.footage) return true;
  const first = j.frame - (j.span ?? 1);
  return clipCuts.some((c) => c >= first - 1 && c <= j.frame + 1);
}

/**
 * D306: the timeline's straight cuts between footage: frames where a visible clip (video or picture)
 * ends and another begins, with no graphic starting or ending within a frame of it (a graphic cutting
 * there is the graphic's cut). `starts` maps an item to its start frame.
 */
export function clipCutFrames(tracks: ReadonlyArray<{ readonly hidden?: boolean; readonly items: ReadonlyArray<{ readonly id: string; readonly type: string; readonly duration: number }> }>, starts: ReadonlyMap<string, number>): Array<number> {
  const items = tracks.filter((t) => !t.hidden).flatMap((t) => t.items);
  const edges = (type: string, at: "start" | "end") => new Set(items.filter((i) => i.type === type && starts.has(i.id)).map((i) => starts.get(i.id)! + (at === "end" ? i.duration : 0)));
  const ends = edges("clip", "end"), begins = edges("clip", "start");
  const graphic = [...edges("graphic", "start"), ...edges("graphic", "end")];
  return [...begins].filter((f) => f > 0 && ends.has(f) && !graphic.some((g) => Math.abs(g - f) <= 1)).sort((a, b) => a - b);
}

/** A nearly empty run (frames, inclusive) and, when its edges aren't exact, how far it may reach. */
export interface EmptyRun {
  readonly start: number;
  readonly end: number;
  readonly upto?: { readonly start: number; readonly end: number };
}
const runSeconds = (r: { readonly start: number; readonly end: number }, fps: number) => (r.end + 1 - r.start) / fps;

/** The scan's nearly empty runs of EMPTY_MIN_S or more (a run counts its last frame), or that may reach it (upto). */
export const emptyRuns = (scan: MotionScan, fps: number) => scan.empty.filter((e) => Math.max(runSeconds(e, fps), e.upto ? runSeconds(e.upto, fps) : 0) >= EMPTY_MIN_S - 1e-9);
/** The scan's low-information runs of EMPTY_MIN_S or more (or that may reach it), the longest first. */
export const thinRuns = (scan: MotionScan, fps: number) =>
  (scan.thin ?? []).filter((e) => Math.max(runSeconds(e, fps), e.upto ? runSeconds(e.upto, fps) : 0) >= EMPTY_MIN_S - 1e-9).sort((a, b) => runSeconds(b, fps) - runSeconds(a, fps));

/** What a scan cut short by the time budget says (in the facts and the tool's result). */
export const budgetLine = (covered: number, planned: number) => `The frame-difference scan covered ${covered} of ${planned} frames: time budget (the frames around each scene change first, then the rest in order).`;

/** The scan for the critic, as facts (in <motion>). */
export function motionLines(scan: MotionScan, fps: number, scanned: { readonly consecutive: boolean; readonly planned?: number; readonly covered?: number; readonly clipCuts?: ReadonlyArray<number> }): Array<string> {
  const covered = scanned.covered ?? scan.frames;
  const lines = [
    ...(scanned.planned !== undefined && covered < scanned.planned ? [budgetLine(covered, scanned.planned)] : []),
    `Frame-difference scan of ${scan.frames} small grey frames (${scanned.consecutive ? "every frame" : "every frame around each scene change, an even sampling of the rest, and each big change between two samples narrowed down to the frame it happens on"}); a typical frame-to-frame change is ${(scan.typical * 100).toFixed(2)} %.`];
  lines.push(scan.pops.length ? `Single-frame pops and flashes (a few frames unlike the frames on both sides, the picture going back): ${scan.pops.map((p) => `f ${p.frame}${p.span ? ` to f ${p.frame + p.span - 1}` : ""} (${secs(p.frame, fps)}, ${p.ratio}×)`).join(", ")}.` : "No single-frame pops or flashes found.");
  const cutLabel = (j: MotionEvent) => `f ${j.frame} (${secs(j.frame, fps)}, ${(j.diff * 100).toFixed(0)} %${j.span ? `, over ${j.span} frames` : ""})`;
  const footageCuts = scan.jumps.filter((j) => footageJump(j, scanned.clipCuts));
  const otherCuts = scan.jumps.filter((j) => !footageJump(j, scanned.clipCuts));
  if (otherCuts.length) lines.push(`One-frame jumps and quick swaps (cuts, or changes that took one frame or under ${BURST_S} s between still frames): ${otherCuts.slice(0, 8).map(cutLabel).join(", ")}.`);
  // D306: the user's call: straight cuts between drone shots read cleaner than shape transitions.
  if (footageCuts.length) lines.push(`Cuts between two footage shots (the changed picture is photographic on both sides, or the timeline cuts from one clip to the next there): ${footageCuts.slice(0, 8).map(cutLabel).join(", ")}. A straight cut between footage shots is fine even in a one-take film: never a motion defect or a blocker; judge only whether it lands on the beat.`);
  lines.push(scan.holds.length ? `Holds over 1 s with nothing changing: ${scan.holds.map((h) => `f ${h.start} to f ${h.end}`).join(", ")}.` : "No hold over 1 s.");
  const empties = emptyRuns(scan, fps);
  if (scan.emptyMeasured) lines.push(empties.length ? `Nearly empty frames (bare canvas with at most a cursor or a speck) for ${EMPTY_MIN_S} s or more: ${empties.map((e) => (runSeconds(e, fps) >= EMPTY_MIN_S - 1e-9 ? `f ${e.start} to f ${e.end} (${secs(e.start, fps)}, ${runSeconds(e, fps).toFixed(2)} s)` : `f ${e.start} to f ${e.end} empty, maybe f ${e.upto!.start} to f ${e.upto!.end} (${secs(e.upto!.start, fps)}, up to ${runSeconds(e.upto!, fps).toFixed(2)} s; edges not narrowed)`)).join(", ")}.` : "No nearly empty stretch.");
  const thins = thinRuns(scan, fps);
  if (scan.thinMeasured) lines.push(thins.length ? `Low-information frames (${Math.round(THIN_FLAT * 100)} % or more one flat colour, nothing taller than ${+(THIN_TALL * 100).toFixed(1)} % of the height: a near-empty panel with a line of small text) for ${EMPTY_MIN_S} s or more: ${thins.map((e) => (runSeconds(e, fps) >= EMPTY_MIN_S - 1e-9 ? `f ${e.start} to f ${e.end} (${secs(e.start, fps)}, ${runSeconds(e, fps).toFixed(2)} s)` : `f ${e.start} to f ${e.end}, maybe f ${e.upto!.start} to f ${e.upto!.end} (up to ${runSeconds(e.upto!, fps).toFixed(2)} s; edges not narrowed)`)).join(", ")}.` : "No low-information stretch.");
  if (scan.fast.length) lines.push(`Very fast moves (${Math.round(FAST_STEP * 100)} % or more of the picture changing every frame for ${FAST_RUN}+ frames; the export's motion blur may stack them into copies; the other frames here are sharp, so judge that on the strips drawn with the project's motion blur when they are among the images): ${scan.fast.slice(0, 6).map((m) => `f ${m.start} to f ${m.end}`).join(", ")}.`);
  if (scan.open) lines.push(`The open: ${scan.open.frame === undefined ? "no real change in the first 2 s" : `the first real change (${Math.round(OPEN_MOVE * 100 * 10) / 10} % of the picture in ${OPEN_WINDOW_S} s) lands at f ${scan.open.frame} (${secs(scan.open.frame, fps)})`}${scan.open.typing !== undefined ? `; typing shows from f ${scan.open.typing} (${secs(scan.open.typing, fps)})` : ""}${openLate(scan, fps) ? ", a static open" : ""}.`);
  if (scan.seam) lines.push(`Loop seam: the last frame differs from frame 0 by ${(scan.seam.diff * 100).toFixed(1)} %${seamBroken(scan) ? ", a visible jump if it loops" : ", a clean loop"}.`);
  return lines;
}

/**
 * Grey frames of `w` × `h` pixels from JPEGs, decoded by one ffmpeg run (the JPEGs piped in as an
 * image2pipe stream). Throws when ffmpeg fails or returns the wrong size.
 */
/** The scan's grey size for a canvas: SCAN_WIDTH wide, even height. */
export const scanSize = (canvas: { readonly width: number; readonly height: number }) => ({ w: SCAN_WIDTH, h: Math.max(2, Math.round((SCAN_WIDTH * canvas.height) / canvas.width / 2) * 2) });

/** A strip's sheet: one row of `count` cells `cell` px wide (STRIP_CELL) with their captions (a downscaled image per strip). */
export function stripLayout(count: number, aspect: number, cell = STRIP_CELL): SheetLayout {
  const a = aspect > 0 && Number.isFinite(aspect) ? aspect : 16 / 9;
  const n = Math.max(1, count);
  const cellHeight = Math.max(1, Math.round(cell / a));
  return { columns: n, rows: 1, cellWidth: cell, cellHeight, width: n * cell + GAP * (n + 1), height: cellHeight + CAPTION_HEIGHT + GAP * 2 };
}

/**
 * Round 2's A1: the export's motion blur (12 sub-frames) ghosted the fast morph at 5.9 s and the
 * drop at 12.1 s, but the critic only saw sharp frames. At the scan's fast moves (at most BLUR_RUNS)
 * a strip of their first, middle and last frame drawn with the project's blur, BLUR_CELL px wide.
 */
export const BLUR_RUNS = 3;
export const BLUR_CELL = 360;
export function blurPicks(fast: MotionScan["fast"], max = BLUR_RUNS): Array<{ readonly start: number; readonly end: number; readonly frames: ReadonlyArray<number> }> {
  return fast.slice(0, max).map((m) => ({ start: m.start, end: m.end, frames: [...new Set([m.start, Math.round((m.start + m.end) / 2), m.end])] }));
}

/**
 * Whether the film is meant as one take (no cuts, no holds: the motion-film skill, or the brief's
 * words) and as a loop (the brief's words only, since the skill no longer prescribes one). D296: and whether it is footage first (the brief's words name footage, or the
 * timeline has video or pictures), where the footage should fill the frame.
 */
/** Words that make a film UI first: it shows an app's interface. */
const UI_WORDS = /\b(app|UI|interface|screens?|composer|timeline|cursor|software|the program)\b/i;

/**
 * Phrases that ask for one take. "cut off/out/into" is cropping or trimming, not an edit cut;
 * "biliup" is the app's name; "one-shot" usually means one prompt. English only: other languages aren't detected (the skill or the brief
 * still make the film one take, with findings rather than blockers).
 */
const ONE_TAKE = /\b(?:one|single)[- ]take\b|\boner\b|\b(?:one|single) (?:continuous|unbroken) (?:shot|take|move|camera move)\b|\bcontinuous (?:shot|take)\b|\b(?:never|don'?t|do not) cut\b(?! ?(?:off|out|into|down|back|short))|\b(?:no|zero|without(?: any)?) (?:hard )?cuts?\b|\bwith no (?:hard )?cuts?\b/gi;
/** Before a phrase in its clause: it is negated ("not one take", "doesn't need to be one take"). */
export const NEGATED = /\b(?:not|no need|needn'?t|isn'?t|aren'?t|don'?t|doesn'?t|do not|does not|won'?t|instead of|rather than|forget|skip|drop|avoid|nor)\b[^.;!?\n]{0,30}$/i;
/** An alternative or a release: "one take or cuts, your call", "cuts are fine", "up to you". */
const OPTIONAL = /\bor (?:hard )?cuts?\b|\bcuts? or\b|\bcuts? (?:are|is) (?:fine|ok(?:ay)?|allowed|welcome)\b|\byour call\b|\bup to you\b|\beither way\b|\bif you (?:want|like|prefer)\b|\boptional(?:ly)?\b/i;

/** A request's clauses: sentences, and the turns "but", "actually", "however" start. */
export const clauses = (text: string): Array<string> => text.split(/(?<=[.;!?\n])\s*|\s+(?:but|actually|however)\s+/i);

/**
 * Whether these words ask for one take: some clause has a one-take phrase that isn't negated or
 * offered as one option. Conservative (a miss leaves cuts as findings; a false ask fails rounds):
 * the last clause that speaks to it wins, so "one take" then "actually, cuts are fine" is no ask.
 */
export function asksOneTake(text: string): boolean {
  let verdict = false;
  for (const clause of clauses(text)) {
    const hits = [...clause.matchAll(ONE_TAKE)];
    const release = OPTIONAL.test(clause) || /\b(?:add|use|with) (?:a |some )?(?:hard )?cuts?\b|\bcuts? (?:are|is) (?:fine|ok(?:ay)?|allowed)\b/i.test(clause);
    if (!hits.length) {
      if (release && /\bcuts?\b/i.test(clause)) verdict = false;
      continue;
    }
    verdict = !release && hits.some((m) => !NEGATED.test(clause.slice(0, m.index)));
  }
  return verdict;
}

/** Footage named in a request: the shots a cut goes between. */
// "shot" and "take" alone aren't footage: "one continuous shot, cut on the beat" asks for one take.
const FOOTAGE_NOUN = /\b(?:footage|drone|clips?|shots|b-?roll|aerials?|videos)\b/i;
/** A cut as an edit: "cut", "cuts", "cutting", "chop", "intercut", "beat cuts". */
const CUT_VERB = /\b(?:cut|cuts|cutting|chop(?:ped|ping)?|intercut)\b/i;
/** The beat, the music it lands on. */
const BEAT = /\b(?:beats?|rhythm|music|song|track|tempo|drums?|bpm)\b/i;

/**
 * D306: whether these words ask for the footage cut on the beat: some clause names footage, a cut and
 * the beat (or the music) together, not negated ("cut the drone clips to the song", "footage cut on
 * the beat", "beat cuts between the drone shots"). Then cuts between footage shots are what the user
 * asked for, and only the user's own one-take words make the other cuts block.
 */
export function asksFootageCuts(text: string): boolean {
  return clauses(text).some((clause) => {
    const cut = CUT_VERB.exec(clause);
    if (!cut || !FOOTAGE_NOUN.test(clause) || !BEAT.test(clause)) return false;
    return !NEGATED.test(clause.slice(0, cut.index)) && !/\b(?:don'?t|do not|never|no)\s+cut/i.test(clause);
  });
}

/**
 * What the film is (the brief's words, the active skills) and what the user asked for in their own
 * words (`request`): `asked` when the request itself asks for one take. A1-noskill3 asked for "one
 * take on the beat" with the motion-film skill off, and its four hard cuts were findings only: the
 * request decides that cuts fail the round, whatever skills are on (hard in motionFindings).
 */
export function motionIntent(o: { readonly skills: ReadonlyArray<string>; readonly words: string; readonly hasFootage?: boolean; readonly request?: string }): { oneTake: boolean; loop: boolean; footage: boolean; asked: boolean; footageCuts: boolean } {
  const film = o.skills.includes("motion-film");
  const asked = asksOneTake(o.request ?? "");
  return {
    asked,
    footageCuts: asksFootageCuts(o.request ?? ""),
    oneTake: film || asked || asksOneTake(o.words),
    loop: /\bloop(s|ing|ed)?\b|last frame (is|=|equals) (the )?first/i.test(o.words),
    // Rehearsal 3: a UI-first film names its footage too (the preview plays it), but its frames are a
    // calm canvas around one UI element by design, so its words naming the app's UI keep it UI first.
    footage: !!o.hasFootage || (/\b(footage|drone|b-?roll|clips?|travel|photos?|aerials?)\b/i.test(o.words) && !UI_WORDS.test(o.words)),
  };
}

// ---- D296: scale (how much of the frame is flat background) and the storyboard's words ----

/** A footage-first film's frames should be at most this share flat background at most beats. */
export const FLAT_MAX = 0.35;
/** Two grey levels this close count as the same flat colour; a pixel this close to its right and lower neighbours is flat. */
const FLAT_LEVELS = 6;
const FLAT_GRADIENT = 3;

/**
 * The share of a grey frame (w × h) that is flat background: pixels in a flat neighbourhood (each
 * within FLAT_GRADIENT of its right and lower neighbours) whose level is the frame's most common
 * flat level (± FLAT_LEVELS). A card of footage on black is about the black's share; footage
 * full-bleed is near 0 (it has texture); a flat sky counts as flat. 0..1.
 */
export function flatFraction(gray: Uint8Array, w: number, h: number): number {
  const n = w * h;
  if (!n || gray.length < n) return 0;
  const flat = new Uint8Array(n);
  const hist = new Uint32Array(256);
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      const v = gray[i]!;
      const r = x + 1 < w ? gray[i + 1]! : gray[i - 1]!;
      const d = y + 1 < h ? gray[i + w]! : gray[i - w]!;
      if (Math.abs(v - r) <= FLAT_GRADIENT && Math.abs(v - d) <= FLAT_GRADIENT) {
        flat[i] = 1;
        hist[v]!++;
      }
    }
  // The most common flat level (summed over ± FLAT_LEVELS).
  let best = 0;
  let level = 0;
  for (let v = 0; v < 256; v++) {
    let c = 0;
    for (let u = Math.max(0, v - FLAT_LEVELS); u <= Math.min(255, v + FLAT_LEVELS); u++) c += hist[u]!;
    if (c > best) {
      best = c;
      level = v;
    }
  }
  let count = 0;
  for (let i = 0; i < n; i++) if (flat[i] && Math.abs(gray[i]! - level) <= FLAT_LEVELS) count++;
  return count / n;
}

export interface FlatStrip {
  readonly label: string;
  readonly at: number;
  /** The median flat share of the strip's frames. */
  readonly flat: number;
  /** The median share of flat canvas reaching the frame's edge (canvasShare), and its grey level. */
  readonly canvas?: number;
  readonly level?: number;
}

/**
 * The canvas around the picture: from the frame's edge, the pixels of the edge's most common flat
 * level (± FLAT_LEVELS, each within FLAT_GRADIENT of the one it was reached from), and that level.
 * A dark panel on near-black with no visible edge counts as canvas (it reads as one), a full-bleed
 * picture is about 0, a hero element on a calm canvas is the canvas's share. 0..1, level 0..255.
 */
export function canvasShare(gray: Uint8Array, w: number, h: number): { share: number; level: number } {
  const c = canvasFill(gray, w, h);
  return c ? { share: c.count / (w * h), level: c.level } : { share: 0, level: 0 };
}

/** canvasShare's flood: which pixels are the canvas, how many, its level; undefined for no frame. */
function canvasFill(gray: Uint8Array, w: number, h: number): { seen: Uint8Array; count: number; level: number } | undefined {
  const n = w * h;
  if (!n || gray.length < n) return undefined;
  const edge: Array<number> = [];
  for (let x = 0; x < w; x++) edge.push(x, (h - 1) * w + x);
  for (let y = 1; y < h - 1; y++) edge.push(y * w, y * w + w - 1);
  const hist = new Uint32Array(256);
  for (const i of edge) hist[gray[i]!]!++;
  let best = 0;
  let level = 0;
  for (let v = 0; v < 256; v++) {
    let c = 0;
    for (let u = Math.max(0, v - FLAT_LEVELS); u <= Math.min(255, v + FLAT_LEVELS); u++) c += hist[u]!;
    if (c > best) {
      best = c;
      level = v;
    }
  }
  // The most common single level inside that window (a tie in the window sum leans to its low end).
  let peak = level;
  for (let u = Math.max(0, level - FLAT_LEVELS); u <= Math.min(255, level + FLAT_LEVELS); u++) if (hist[u]! > hist[peak]!) peak = u;
  level = peak;
  const seen = new Uint8Array(n);
  const queue: Array<number> = [];
  for (const i of edge) if (!seen[i] && Math.abs(gray[i]! - level) <= FLAT_LEVELS) (seen[i] = 1), queue.push(i);
  let count = 0;
  while (queue.length) {
    const i = queue.pop()!;
    count++;
    const x = i % w;
    for (const j of [x + 1 < w ? i + 1 : -1, x > 0 ? i - 1 : -1, i + w < n ? i + w : -1, i - w >= 0 ? i - w : -1]) {
      if (j < 0 || seen[j]) continue;
      if (Math.abs(gray[j]! - level) <= FLAT_LEVELS && Math.abs(gray[j]! - gray[i]!) <= FLAT_GRADIENT) (seen[j] = 1), queue.push(j);
    }
  }
  return { seen, count, level };
}

/** What a frame shows besides its canvas: the share that isn't canvas, in how many separate marks, and whether that's nearly empty. */
export interface EmptyMeasure {
  readonly content: number;
  readonly parts: number;
  readonly empty: boolean;
}

/**
 * How empty a grey frame (w × h) is: the pixels that aren't the canvas (canvasShare's flood from the
 * edge) as a share, and how many separate marks they make (8-connected; counted up to 64). Nearly
 * empty: EMPTY_CONTENT or less in EMPTY_PARTS marks or fewer (a cursor, a speck). A dark UI's text,
 * a wordmark's letters and a headline are many marks or a large share. Best at EMPTY_WIDTH.
 */
export function emptyMeasure(gray: Uint8Array, w: number, h: number): EmptyMeasure {
  const c = canvasFill(gray, w, h);
  if (!c) return { content: 1, parts: 0, empty: false };
  const n = w * h;
  const content = (n - c.count) / n;
  const seen = c.seen;
  let parts = 0;
  const stack: Array<number> = [];
  for (let s = 0; s < n && parts < 64; s++) {
    if (seen[s]) continue;
    parts++;
    seen[s] = 1;
    stack.push(s);
    while (stack.length) {
      const i = stack.pop()!;
      const x = i % w, y = (i - x) / w;
      for (let dy = -1; dy <= 1; dy++)
        for (let dx = -1; dx <= 1; dx++) {
          const xx = x + dx, yy = y + dy;
          if (xx < 0 || yy < 0 || xx >= w || yy >= h) continue;
          const j = yy * w + xx;
          if (!seen[j]) (seen[j] = 1), stack.push(j);
        }
    }
  }
  return { content: round(content), parts, empty: content <= EMPTY_CONTENT && parts <= EMPTY_PARTS };
}

/** How much a frame says (thinMeasure): its flat colour family's share, its tallest readable mark (share of the height), and whether that's low-information. */
export interface ThinMeasure {
  readonly flat: number;
  readonly tall: number;
  readonly thin: boolean;
}

/**
 * A grey frame's (w × h) information (THIN_* above): the share within THIN_LEVELS of its most
 * common level, and the tallest mark THIN_CONTRAST or more off that level (8-connected, gaps of up
 * to 2 px in a row joined, so letters make words) at least THIN_MARK_W of the short side wide (a cursor
 * isn't a mark). Low-information: THIN_FLAT or flatter, a mark there, all of them under THIN_TALL.
 * A frame with no mark at all is the empty rule's (emptyMeasure). Best at THIN_WIDTH.
 */
export function thinMeasure(gray: Uint8Array, w: number, h: number): ThinMeasure {
  const n = w * h;
  if (!n || gray.length < n) return { flat: 0, tall: 1, thin: false };
  const hist = new Uint32Array(256);
  for (let i = 0; i < n; i++) hist[gray[i]!]!++;
  let best = 0;
  let level = 0;
  for (let v = 0; v < 256; v++) {
    let c = 0;
    for (let u = Math.max(0, v - THIN_LEVELS); u <= Math.min(255, v + THIN_LEVELS); u++) c += hist[u]!;
    if (c > best) (best = c), (level = v);
  }
  const flat = best / n;
  if (flat < THIN_FLAT) return { flat: round(flat), tall: 1, thin: false };
  const mark = new Uint8Array(n);
  for (let y = 0; y < h; y++) {
    let last = -1;
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      if (Math.abs(gray[i]! - level) < THIN_CONTRAST) continue;
      mark[i] = 1;
      if (last >= 0 && x - last > 1 && x - last <= 3) for (let k = last + 1; k < x; k++) mark[y * w + k] = 1;
      last = x;
    }
  }
  let tall = 0;
  let marks = 0;
  const minW = THIN_MARK_W * Math.min(w, h);
  const stack: Array<number> = [];
  for (let s = 0; s < n; s++) {
    if (mark[s] !== 1) continue;
    let x0 = w, x1 = -1, y0 = h, y1 = -1;
    mark[s] = 2;
    stack.push(s);
    while (stack.length) {
      const i = stack.pop()!;
      const x = i % w, y = (i - x) / w;
      if (x < x0) x0 = x;
      if (x > x1) x1 = x;
      if (y < y0) y0 = y;
      if (y > y1) y1 = y;
      for (let dy = -1; dy <= 1; dy++)
        for (let dx = -1; dx <= 1; dx++) {
          const xx = x + dx, yy = y + dy;
          if (xx < 0 || yy < 0 || xx >= w || yy >= h) continue;
          const j = yy * w + xx;
          if (mark[j] === 1) (mark[j] = 2), stack.push(j);
        }
    }
    if (x1 - x0 + 1 < minW) continue;
    marks++;
    tall = Math.max(tall, (y1 - y0 + 1) / h);
  }
  return { flat: round(flat), tall: round(tall), thin: marks > 0 && tall < THIN_TALL };
}

/**
 * The low-information grey size for a canvas: THIN_WIDTH on its short side, the long side even. A
 * 16:9 canvas THIN_WIDTH wide was 112 px tall, so a line's height came in 0.9 % steps and a 2.4 %
 * line read as 2.7 %; scanRenderWidth renders its frames this big.
 */
export const thinSize = (canvas: { readonly width: number; readonly height: number }) => {
  const even = (n: number) => Math.max(2, Math.round(n / 2) * 2);
  return canvas.width > canvas.height ? { w: even((THIN_WIDTH * canvas.width) / canvas.height), h: THIN_WIDTH } : { w: THIN_WIDTH, h: even((THIN_WIDTH * canvas.height) / canvas.width) };
};
/** The width the strips' and scan's frames are rendered at: STRIP_CELL, or wider for a landscape canvas (thinSize). */
export const scanRenderWidth = (canvas: { readonly width: number; readonly height: number }) => Math.max(STRIP_CELL, thinSize(canvas).w);

/** The emptiness grey size for a canvas: EMPTY_WIDTH wide, even height. */
export const emptySize = (canvas: { readonly width: number; readonly height: number }) => ({ w: EMPTY_WIDTH, h: Math.max(2, Math.round((EMPTY_WIDTH * canvas.height) / canvas.width / 2) * 2) });

/** The scale facts for the critic: each strip's flat share, and whether most are over FLAT_MAX. */
export function flatLines(strips: ReadonlyArray<FlatStrip>, fps: number, footage: boolean): Array<string> {
  if (!strips.length) return [];
  const over = strips.filter((x) => x.flat > FLAT_MAX);
  return [
    `Flat background (one flat colour, no footage, picture or type) per frame strip, the median of its frames: ${strips.map((x) => `${x.label} (${secs(x.at, fps)}) ${Math.round(x.flat * 100)} %`).join(", ")}.`,
    strips.some((x) => x.canvas !== undefined)
      ? `Canvas around the picture (flat colour reaching the frame's edge; grey 0 black to 255 white) per strip: ${strips.flatMap((x) => (x.canvas === undefined ? [] : [`${x.label} ${Math.round(x.canvas * 100)} % at grey ${x.level ?? 0}`])).join(", ")}. A calm canvas around one hero element is fine; a dark canvas (grey under 40) around dark panels reads muddy.`
      : "",
    footage
      ? over.length * 2 > strips.length
        ? `Over ${Math.round(FLAT_MAX * 100)} % at ${over.length} of ${strips.length} strips: in a footage-first film the footage sits small on an empty canvas there.`
        : `At most ${Math.round(FLAT_MAX * 100)} % at most strips: the footage fills the frame.`
      : "",
  ].filter(Boolean);
}

/** A finding (rule "scale") when a footage-first film's frames are mostly flat background at more than half its strips. */
export function flatFindings(strips: ReadonlyArray<FlatStrip>, fps: number, footage: boolean): Array<CriticFinding> {
  const over = strips.filter((x) => x.flat > FLAT_MAX);
  if (!footage || over.length * 2 <= strips.length) return [];
  const worst = [...over].sort((a, b) => b.flat - a.flat)[0]!;
  return [{ rule: "scale", key: "scale:flat", frame: worst.at, hard: false, message: `The footage sits small on flat background: over ${Math.round(FLAT_MAX * 100)} % of the frame is one flat colour at ${over.length} of ${strips.length} scene changes (worst ${Math.round(worst.flat * 100)} % at ${secs(worst.at, fps)}). At rest the hero shape should fill most of the frame (70 % or more, margins of about 6 %), flood past the corners on the big beats, and the type sit large inside or over it.` }];
}

/** The words a beat says it shows: text in quotes in its title, entry and exit ("…", '…', “…”, ‘…’), at most 4. */
export function beatWords(beat: { readonly title?: string; readonly entry?: string; readonly exit?: string }): Array<string> {
  const text = [beat.title, beat.entry, beat.exit].filter(Boolean).join("\n");
  const out: Array<string> = [];
  for (const m of text.matchAll(/"([^"\n]{2,120})"|“([^”\n]{2,120})”|‘([^’\n]{2,120})’|(?:^|[\s(:→])'([^'\n]{2,120})'(?=[\s.,;:)→]|$)/g)) {
    const w = (m[1] ?? m[2] ?? m[3] ?? m[4] ?? "").trim();
    if (w && /[A-Za-z0-9]/.test(w) && !out.includes(w)) out.push(w);
  }
  return out.slice(0, 4);
}

/** The storyboard's words per beat for the critic to read in that beat's frames (D296), or none. */
export function beatWordLines(beats: ReadonlyArray<{ readonly start: number; readonly end: number; readonly title?: string; readonly entry?: string; readonly exit?: string }> | undefined): Array<string> {
  const lines = (beats ?? []).flatMap((b, i) => {
    const words = beatWords(b);
    return words.length ? [`beat ${i + 1} (${b.start} s to ${b.end} s): ${words.map((w) => `"${w}"`).join(", ")}`] : [];
  });
  return lines.length ? ["The storyboard's words, each beat's text as approved; read them in that beat's frames (its strip and its last frame): missing, cut or different words are a defect.", ...lines] : [];
}
