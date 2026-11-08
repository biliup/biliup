// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Springs (D192): a damped spring's step response in closed form, a pure function of time, so the
// preview, the export and the agent's frame renders all draw the same value at a frame whatever
// order they ask in (no state, no integration).
//
// The spring moves a value from 0 to 1: m·x'' + c·x' + k·(x − 1) = 0 with x(0) = 0 and
// x'(0) = velocity (in "distances per second"). With ω0 = √(k/m) and ζ = c / (2√(k·m)) it is
// under-damped (ζ < 1: it overshoots and oscillates), critically damped (ζ = 1) or over-damped
// (ζ > 1: it creeps in). Time is in seconds.
//
// Where they're used:
// - box keys (box.ts keyAt): a key's ease may name a preset; the move from that key to the next one
//   is a real-time spring that starts at the key's frame and targets the next key's value
//   ("sum of springs": a later key that starts before it settled adds its own spring on top, the
//   earlier one's rest still plays out, so the motion stays continuous and keeps its velocity);
// - in/out moves (timeline.ts animationAt): params.ease may name a preset; the spring is stretched
//   so that its settle time is the move's duration (springEase).

export interface SpringParams {
  /** k: how hard it pulls toward the target. */
  readonly stiffness: number;
  /** c: how much it resists moving (friction). */
  readonly damping: number;
  /** m: how heavy the moving value is (default 1). */
  readonly mass?: number;
  /** Initial velocity in distances per second (default 0). */
  readonly velocity?: number;
}

export type SpringName = "spring" | "spring.snap" | "spring.settle" | "spring.land" | "spring.bounce";

/**
 * The presets (mass 1, at rest): snap (UI snap: stiff, near critical, overshoot well under 1 %),
 * settle (a panel settling: softer, about 2 % over), land (a headline landing: one visible
 * overshoot of about 11 %, then it settles), bounce (bouncy: a few visible oscillations, about 43 %
 * over at first). "spring" is settle.
 */
export const SPRING_PRESETS: Readonly<Record<SpringName, SpringParams>> = {
  "spring.snap": { stiffness: 400, damping: 36 },
  "spring.settle": { stiffness: 170, damping: 20 },
  "spring.land": { stiffness: 220, damping: 17 },
  "spring.bounce": { stiffness: 300, damping: 9 },
  spring: { stiffness: 170, damping: 20 },
};

export const SPRING_NAMES = Object.keys(SPRING_PRESETS) as ReadonlyArray<SpringName>;

export const isSpringName = (v: unknown): v is SpringName => typeof v === "string" && Object.hasOwn(SPRING_PRESETS, v);

/** For the agent's system prompt (D192). */
export const SPRING_DOC =
  'Springs: wherever an ease is taken (a key\'s "ease", a move\'s params.ease such as move.zoom, move.rise, move.pop or move.scale) it may also be "spring.snap" (stiff, no visible overshoot, UI snaps), "spring.settle" or "spring" (soft, slight overshoot, panels), "spring.land" (one visible overshoot then rest, headlines landing) or "spring.bounce" (bouncy, a few oscillations); on keys the spring runs in real time from its key toward the next key\'s value and keeps settling past it (a later key retargets it smoothly), in a move it is fitted to the move\'s duration, e.g. "keys":[{"frame":0,"scale":0.8,"ease":"spring.land"},{"frame":20,"scale":1}].';

const params = (p: SpringName | SpringParams): Required<SpringParams> => {
  const s = typeof p === "string" ? SPRING_PRESETS[p] : p;
  const pos = (n: unknown, d: number) => (typeof n === "number" && Number.isFinite(n) && n > 0 ? n : d);
  return { stiffness: pos(s.stiffness, 170), damping: typeof s.damping === "number" && Number.isFinite(s.damping) && s.damping >= 0 ? s.damping : 20, mass: pos(s.mass, 1), velocity: typeof s.velocity === "number" && Number.isFinite(s.velocity) ? s.velocity : 0 };
};

/** Position and velocity of the spring at `t` seconds (0 → 1; t ≤ 0 is the start). */
export function springState(spring: SpringName | SpringParams, t: number): { readonly value: number; readonly velocity: number } {
  const { stiffness: k, damping: c, mass: m, velocity: v0 } = params(spring);
  if (!(t > 0)) return { value: 0, velocity: v0 };
  const w0 = Math.sqrt(k / m);
  const zeta = c / (2 * Math.sqrt(k * m));
  // y = x − 1: y(0) = −1, y'(0) = v0.
  if (Math.abs(zeta - 1) < 1e-6) {
    const b = v0 - w0;
    const e = Math.exp(-w0 * t);
    const y = (-1 + b * t) * e;
    return { value: 1 + y, velocity: (b - w0 * (-1 + b * t)) * e };
  }
  if (zeta < 1) {
    const wd = w0 * Math.sqrt(1 - zeta * zeta);
    const a = -1;
    const b = (v0 + zeta * w0 * a) / wd;
    const e = Math.exp(-zeta * w0 * t);
    const cos = Math.cos(wd * t);
    const sin = Math.sin(wd * t);
    const y = e * (a * cos + b * sin);
    return { value: 1 + y, velocity: e * ((b * wd - zeta * w0 * a) * cos - (a * wd + zeta * w0 * b) * sin) };
  }
  const root = w0 * Math.sqrt(zeta * zeta - 1);
  const r1 = -zeta * w0 + root;
  const r2 = -zeta * w0 - root;
  const c2 = (v0 + r1) / (r2 - r1);
  const c1 = -1 - c2;
  const e1 = Math.exp(r1 * t);
  const e2 = Math.exp(r2 * t);
  return { value: 1 + c1 * e1 + c2 * e2, velocity: c1 * r1 * e1 + c2 * r2 * e2 };
}

/** The spring's value at `t` seconds: 0 at the start, settling at 1. */
export const springAt = (spring: SpringName | SpringParams, t: number): number => springState(spring, t).value;

const settleCache = new Map<string, number>();

/**
 * When the spring has settled (seconds): from then on it stays within `tolerance` of 1 and its
 * velocity under `tolerance`·ω0. Deterministic (a fine scan over the closed form), memoised.
 */
export function springSettleTime(spring: SpringName | SpringParams, tolerance = 1e-3): number {
  const p = params(spring);
  const key = `${p.stiffness}/${p.damping}/${p.mass}/${p.velocity}/${tolerance}`;
  const hit = settleCache.get(key);
  if (hit !== undefined) return hit;
  const w0 = Math.sqrt(p.stiffness / p.mass);
  const zeta = p.damping / (2 * Math.sqrt(p.stiffness * p.mass));
  // The slowest decay rate bounds how far to look.
  const rate = zeta < 1 ? zeta * w0 : zeta === 1 ? w0 : w0 * (zeta - Math.sqrt(zeta * zeta - 1));
  const horizon = Math.min(120, rate > 0 ? (Math.log(1 / tolerance) + Math.log(1 + Math.abs(p.velocity) / w0 + 1) + 8) / rate : 120);
  const dt = Math.min(1 / 1000, horizon / 20000);
  let last = 0;
  for (let t = 0; t <= horizon; t += dt) {
    const s = springState(p, t);
    if (Math.abs(s.value - 1) > tolerance || Math.abs(s.velocity) > tolerance * w0) last = t;
  }
  const out = Math.min(horizon, last + dt);
  settleCache.set(key, out);
  return out;
}

/**
 * A spring as an ease over a share p (0 → 1) of a move (D192): the spring stretched so that its
 * settle time is the whole move, corrected so it ends exactly at 1 (the correction is at most the
 * settle tolerance). It may overshoot past 1 on the way.
 */
export function springEase(spring: SpringName | SpringParams): (p: number) => number {
  const T = springSettleTime(spring);
  const end = springAt(spring, T);
  return (p) => (p <= 0 ? 0 : p >= 1 ? 1 : springAt(spring, p * T) + (1 - end) * p);
}
