// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// General elements (D130, feedback E2): shapes, a progress bar, a lower third, a callout, a
// subscribe button, a countdown, a sticker and an image frame. Like the other components (D76),
// each is a pure function of its props, its box size (canvas px) and the local frame, drawn by the
// preview and the export from the same HTML. Sizes scale with the box; user text is escaped.
import shapeJson from "@/resources/editor/modules/builtin/component.shape.json" with { type: "json" };
import progressBarJson from "@/resources/editor/modules/builtin/component.progress-bar.json" with { type: "json" };
import lowerThirdJson from "@/resources/editor/modules/builtin/component.lower-third.json" with { type: "json" };
import calloutJson from "@/resources/editor/modules/builtin/component.callout.json" with { type: "json" };
import subscribeJson from "@/resources/editor/modules/builtin/component.subscribe.json" with { type: "json" };
import countdownJson from "@/resources/editor/modules/builtin/component.countdown.json" with { type: "json" };
import stickerJson from "@/resources/editor/modules/builtin/component.sticker.json" with { type: "json" };
import imageFrameJson from "@/resources/editor/modules/builtin/component.image-frame.json" with { type: "json" };

import { DEFAULT_THEME, DM_SANS, esc, num, resolveColor, type RenderTarget } from "@/app/lib/editor/core/visual/render";
import type { Draw } from "@/app/lib/editor/core/visual/components";
import { artSvg, stickerArt } from "@/app/lib/editor/core/visual/stickers";
import { fitSize as fitText, inkOn, pointerAt } from "@/app/lib/editor/core/visual/elementsMore";

export { shapeJson, progressBarJson, lowerThirdJson, calloutJson, subscribeJson, countdownJson, stickerJson, imageFrameJson };

const str = (v: unknown) => (typeof v === "string" ? v : "");
const px = (n: number) => `${num(n)}px`;
const numOr = (v: unknown, fallback: number) => (typeof v === "number" && Number.isFinite(v) ? v : fallback);
const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
const color = (target: RenderTarget, v: unknown, fallback: string) => resolveColor(target, v, fallback);
const accent = (target: RenderTarget) => color(target, "accent", DEFAULT_THEME.colors.accent);
const fg = (target: RenderTarget) => color(target, "fg", DEFAULT_THEME.colors.fg);
/** Ease out (cubic). */
const out = (p: number) => 1 - (1 - clamp(p, 0, 1)) ** 3;
/** Ease out with a little overshoot. */
const backOut = (p: number) => {
  const q = clamp(p, 0, 1);
  const c1 = 1.70158;
  return 1 + (c1 + 1) * (q - 1) ** 3 + c1 * (q - 1) ** 2;
};
/** The item's length in frames (at least 1). */
const lengthOf = (d: Draw) => Math.max(1, d.item.duration);
/** A media reference prop ({ media: id }) as a safe image URL in this target, or undefined. */
const imageSrc = (target: RenderTarget, v: unknown) => {
  const id = v && typeof v === "object" ? (v as { media?: unknown }).media : undefined;
  const url = typeof id === "string" ? target.mediaUrl?.(id) : undefined;
  return url && /^(data:image\/|https?:\/\/|biliup-media:)/.test(url) ? esc(url) : undefined;
};
/** Characters, not UTF-16 units. */
const len = (s: string) => [...s].length;
const box = (inner: string, extra = "") => `<div style="position:absolute;inset:0;${extra}">${inner}</div>`;

// ---- shape ----

/** A star's or triangle's outline in a w × h box (with room for its stroke). */
function polygonOf(kind: string, w: number, h: number, inset: number): string {
  const cx = w / 2;
  const cy = h / 2;
  if (kind === "triangle") return `${num(cx)},${num(inset)} ${num(w - inset)},${num(h - inset)} ${num(inset)},${num(h - inset)}`;
  const pts: Array<string> = [];
  const rx = w / 2 - inset;
  const ry = h / 2 - inset;
  for (let i = 0; i < 10; i++) {
    const a = ((-90 + i * 36) * Math.PI) / 180;
    const r = i % 2 ? 0.42 : 1;
    pts.push(`${num(cx + rx * r * Math.cos(a))},${num(cy + 0.06 * ry + ry * r * Math.sin(a))}`);
  }
  return pts.join(" ");
}

export function drawShape({ props, w, h, target }: Draw) {
  const kind = str(props.kind) || "rectangle";
  const fill = color(target, props.fill, accent(target));
  const strokeW = Math.max(0, numOr(props.strokeWidth, 0));
  const stroke = props.stroke !== undefined ? color(target, props.stroke, fill) : undefined;
  if (kind === "line" || kind === "arrow") {
    const c = stroke ?? fill;
    const t = strokeW > 0 ? strokeW : 12;
    const y = h / 2;
    const head = kind === "arrow" ? Math.min(w * 0.4, Math.max(t * 3, h * 0.45)) : 0;
    const end = w - (kind === "arrow" ? head * 0.6 : t / 2);
    const shaft = `<line x1="${num(t / 2)}" y1="${num(y)}" x2="${num(end)}" y2="${num(y)}" stroke="${c}" stroke-width="${num(t)}" stroke-linecap="round"/>`;
    const tip = kind === "arrow" ? `<path d="M${num(w - head)} ${num(y - head / 2)}L${num(w)} ${num(y)}L${num(w - head)} ${num(y + head / 2)}Z" fill="${c}" stroke="${c}" stroke-width="${num(t / 2)}" stroke-linejoin="round"/>` : "";
    return `<svg data-shape="${kind}" width="${num(w)}" height="${num(h)}" viewBox="0 0 ${num(w)} ${num(h)}" style="position:absolute;inset:0;overflow:visible">${shaft}${tip}</svg>`;
  }
  if (kind === "star" || kind === "triangle") {
    const sw = stroke && strokeW > 0 ? strokeW : 0;
    const outline = sw ? ` stroke="${stroke}" stroke-width="${num(sw)}"` : "";
    return `<svg data-shape="${kind}" width="${num(w)}" height="${num(h)}" viewBox="0 0 ${num(w)} ${num(h)}" style="position:absolute;inset:0;overflow:visible"><polygon points="${polygonOf(kind, w, h, sw / 2)}" fill="${fill}"${outline} stroke-linejoin="round"/></svg>`;
  }
  const radius = kind === "circle" ? "50%" : px(clamp(numOr(props.radius, 0), 0, Math.min(w, h) / 2));
  const border = strokeW > 0 && stroke ? `border:${px(strokeW)} solid ${stroke};` : "";
  return `<div data-shape="${kind}" style="position:absolute;inset:0;background:${fill};border-radius:${radius};${border}"></div>`;
}

// ---- progress bar ----

/** How full a progress bar is at `local` frames, 0–1. */
export function progressAt(props: { readonly [key: string]: unknown }, local: number, duration: number): number {
  const value = clamp(numOr(props.value, 100), 0, 100) / 100;
  if (props.animate !== true) return value;
  return value * clamp(local / Math.max(1, duration - 1), 0, 1);
}

export function drawProgressBar(d: Draw) {
  const { props, w, h, target, local } = d;
  const r = px(clamp(numOr(props.radius, h / 2), 0, h / 2));
  const p = progressAt(props, local, lengthOf(d));
  const bar = color(target, props.color, accent(target));
  const track = color(target, props.track, "#FFFFFF33");
  return box(`<div data-progress="${num(p)}" style="position:absolute;left:0;top:0;bottom:0;width:${num(p * 100)}%;background:${bar};border-radius:${r}"></div>`, `background:${track};border-radius:${r};overflow:hidden`);
}

// ---- lower third ----

/** The lower third's looks (D251): the bar (D130's), a pill with an initial, an underline, a light card, a split and plain words. */
export const LOWER_THIRD_STYLES = ["bar", "pill", "underline", "card", "split", "minimal"] as const;

export function drawLowerThird({ props, w, h, target, local, fps }: Draw) {
  const style = (LOWER_THIRD_STYLES as ReadonlyArray<string>).includes(str(props.style)) ? str(props.style) : "bar";
  const k = Math.min(w / 900, h / 200);
  const slide = Math.max(0, numOr(props.slide, 0.5)) * fps;
  const p = slide > 0 ? out(local / slide) : 1;
  // The second line comes a little after the first.
  const p2 = slide > 0 ? out((local - slide * 0.35) / slide) : 1;
  const name = str(props.name);
  const title = str(props.title);
  const bar = color(target, props.accent, accent(target));
  const ink = fg(target);
  const font = `font-family:${DM_SANS}`;
  const nameLine = (size: number, width: number, c: string, extra = "") => (name ? `<div style="font-weight:700;font-size:${px(fitText(name, size, width, 0.55))};line-height:1.05;letter-spacing:-0.01em;white-space:nowrap;color:${c}${extra}">${esc(name)}</div>` : "");
  const titleLine = (size: number, width: number, c: string, extra = "") => (title ? `<div style="font-weight:500;font-size:${px(fitText(title, size, width, 0.52))};line-height:1.2;white-space:nowrap;color:${c};opacity:${num(0.78 * p2)};transform:translateX(${px((1 - p2) * -24 * k)})${extra}">${esc(title)}</div>` : "");
  const column = (left: number, right: number, inner: string, extra = "") => `<div style="position:absolute;left:${px(left)};right:${px(right)};top:0;bottom:0;display:flex;flex-direction:column;justify-content:center;gap:${px(8 * k)};${font};transform:translateX(${px((1 - p) * -40 * k)});opacity:${num(p)}${extra}">${inner}</div>`;
  if (style === "pill") {
    const d = h * 0.7;
    const bg = color(target, props.background, "#0A0A0BCC");
    const textL = (h - d) / 2 + d + 26 * k;
    const width = w - textL - h * 0.4;
    const initial = [...name.trim()][0]?.toUpperCase() ?? "";
    const panel = `<div style="position:absolute;inset:0;background:${bg};border-radius:${px(h / 2)};clip-path:inset(0 ${num((1 - p) * (1 - h / w) * 100)}% 0 0 round ${px(h / 2)})"></div>`;
    const dot = `<div style="position:absolute;left:${px((h - d) / 2)};top:${px((h - d) / 2)};width:${px(d)};height:${px(d)};border-radius:50%;background:${bar};display:flex;align-items:center;justify-content:center;${font};font-weight:800;font-size:${px(d * 0.44)};color:${inkOn(bar)};transform:scale(${num(Math.max(0, backOut(local / Math.max(1, slide * 0.8))))})">${esc(initial)}</div>`;
    return box(panel + dot + column(textL, h * 0.4, nameLine(56 * k, width, ink) + titleLine(30 * k, width, ink), ""));
  }
  if (style === "underline") {
    const width = w - 8 * k;
    const shadow = `;text-shadow:0 2px 12px rgb(0 0 0 / 45%)`;
    const line = `<div style="height:${px(8 * k)};width:${num(Math.min(100, p * 100))}%;max-width:${px(Math.min(width, (name ? fitText(name, 64 * k, width, 0.55) * len(name) * 0.55 : 200 * k) + 20 * k))};background:${bar};border-radius:${px(4 * k)};margin:${px(2 * k)} 0 ${px(4 * k)}"></div>`;
    return box(`<div style="position:absolute;left:0;right:0;top:0;bottom:0;display:flex;flex-direction:column;justify-content:center;gap:${px(6 * k)};${font}">${nameLine(64 * k, width, ink, shadow + `;opacity:${num(p)};transform:translateY(${px((1 - p) * 16 * k)})`)}${line}${titleLine(32 * k, width, ink, shadow)}</div>`);
  }
  if (style === "card") {
    const bg = color(target, props.background, "#FFFFFF");
    const dark = "#0A0A0B";
    const panel = `<div style="position:absolute;inset:0;background:${bg};border-radius:${px(18 * k)};box-shadow:0 ${px(10 * k)} ${px(30 * k)} rgb(0 0 0 / 25%);opacity:${num(p)};transform:translateY(${px((1 - p) * 20 * k)})"></div>`;
    const top = `<div style="position:absolute;left:${px(36 * k)};top:0;width:${px(90 * k)};height:${px(8 * k)};background:${bar};border-radius:0 0 ${px(4 * k)} ${px(4 * k)};transform:scaleX(${num(p)});transform-origin:0 50%"></div>`;
    return box(panel + top + column(36 * k, 36 * k, nameLine(58 * k, w - 72 * k, dark) + titleLine(30 * k, w - 72 * k, dark)));
  }
  if (style === "split") {
    const nameH = title ? h * 0.58 : h;
    const nameSize = fitText(name, nameH * 0.5, w - 60 * k, 0.55);
    const titleSize = fitText(title, (h - nameH) * 0.56, w - 60 * k, 0.52);
    const bg = color(target, props.background, "#0A0A0B");
    const top = `<div style="position:absolute;left:0;top:0;height:${px(nameH)};max-width:100%;padding:0 ${px(30 * k)};display:flex;align-items:center;background:${bar};${font};font-weight:800;font-size:${px(nameSize)};color:${inkOn(bar)};white-space:nowrap;clip-path:inset(0 ${num((1 - p) * 100)}% 0 0)">${esc(name)}</div>`;
    const bottom = title ? `<div style="position:absolute;left:0;top:${px(nameH)};height:${px(h - nameH)};max-width:100%;padding:0 ${px(30 * k)};display:flex;align-items:center;background:${bg};${font};font-weight:600;font-size:${px(titleSize)};color:${ink};white-space:nowrap;clip-path:inset(0 ${num((1 - p2) * 100)}% 0 0)">${esc(title)}</div>` : "";
    return box(top + bottom);
  }
  if (style === "minimal") {
    const shadow = `;text-shadow:0 2px 12px rgb(0 0 0 / 50%)`;
    return box(column(0, 0, nameLine(64 * k, w, ink, shadow) + titleLine(34 * k, w, ink, shadow)));
  }
  const bg = color(target, props.background, "#0A0A0BCC");
  const barW = 14 * k;
  const width = w - barW - 72 * k;
  const panel = `<div style="position:absolute;inset:0;background:${bg};border-radius:${px(10 * k)};clip-path:inset(0 ${num((1 - p) * 100)}% 0 0)"></div>`;
  const accentBar = `<div style="position:absolute;left:0;top:0;bottom:0;width:${px(barW)};background:${bar};border-radius:${px(4 * k)};transform:scaleY(${num(Math.min(1, p * 1.6))});transform-origin:50% 100%"></div>`;
  return box(panel + accentBar + column(barW + 36 * k, 36 * k, nameLine(64 * k, width, ink) + titleLine(34 * k, width, ink)));
}

// ---- callout ----

export function drawCallout({ props, w, h, target, local, fps }: Draw) {
  const arrow = ["left", "right", "up", "down", "none"].includes(str(props.arrow)) ? str(props.arrow) : "left";
  const pill = color(target, props.color, accent(target));
  const ink = color(target, props.text, "#0A0A0B");
  const label = str(props.label);
  const k = Math.min(w / 560, h / 160);
  const horizontal = arrow === "left" || arrow === "right";
  const reach = arrow === "none" ? 0 : horizontal ? Math.min(w * 0.3, 140 * k) : Math.min(h * 0.4, 70 * k);
  const inset = { left: arrow === "left" ? reach : 0, right: arrow === "right" ? reach : 0, top: arrow === "up" ? reach : 0, bottom: arrow === "down" ? reach : 0 };
  const pillH = h - inset.top - inset.bottom;
  const pillW = w - inset.left - inset.right;
  // The words get smaller to fit the pill rather than being cut (D251).
  const size = Math.max(6, Math.min(pillH * 0.42, 52 * k, fitText(label, 52 * k, pillW - pillH * 0.9, 0.56)));
  // The pill pops, then the arrow draws out; the Motion control can disable the entrance.
  const animate = props.pop !== false;
  const pop = animate ? Math.max(0, backOut(local / (0.35 * fps))) : 1;
  const reachP = animate ? out((local - 0.15 * fps) / (0.35 * fps)) : 1;
  const origin = arrow === "left" ? "0% 50%" : arrow === "right" ? "100% 50%" : arrow === "up" ? "50% 0%" : arrow === "down" ? "50% 100%" : "50% 50%";
  const body = `<div style="position:absolute;left:${px(inset.left)};right:${px(inset.right)};top:${px(inset.top)};bottom:${px(inset.bottom)};background:${pill};border-radius:${px(pillH / 2)};display:flex;align-items:center;justify-content:center;padding:0 ${px(Math.min(size * 0.9, pillH * 0.45))};font-family:${DM_SANS};font-weight:700;font-size:${px(size)};color:${ink};white-space:nowrap;overflow:hidden;box-shadow:0 ${px(6 * k)} ${px(20 * k)} rgb(0 0 0 / 22%);transform:scale(${num(pop)});transform-origin:${origin}">${esc(label)}</div>`;
  if (arrow === "none") return box(body);
  const t = Math.max(2, 8 * k);
  const head = Math.max(t * 2.5, 26 * k);
  // The arrow from the pill's edge to the box's edge, pointing out.
  const [x1, y1, x0, y0] = arrow === "left" ? [reach, h / 2, t, h / 2] : arrow === "right" ? [w - reach, h / 2, w - t, h / 2] : arrow === "up" ? [w / 2, reach, w / 2, t] : [w / 2, h - reach, w / 2, h - t];
  const x2 = x1 + (x0 - x1) * reachP;
  const y2 = y1 + (y0 - y1) * reachP;
  const ang = Math.atan2(y0 - y1, x0 - x1);
  const hx = (a: number) => num(x2 - head * Math.cos(ang + a));
  const hy = (a: number) => num(y2 - head * Math.sin(ang + a));
  const svg = reachP > 0 ? `<svg data-arrow="${arrow}" width="${num(w)}" height="${num(h)}" viewBox="0 0 ${num(w)} ${num(h)}" style="position:absolute;inset:0;overflow:visible"><path d="M${num(x1)} ${num(y1)}L${num(x2)} ${num(y2)}M${hx(0.5)} ${hy(0.5)}L${num(x2)} ${num(y2)}L${hx(-0.5)} ${hy(-0.5)}" fill="none" stroke="${pill}" stroke-width="${num(t)}" stroke-linecap="round" stroke-linejoin="round"/></svg>` : `<svg data-arrow="${arrow}" width="0" height="0" style="position:absolute"></svg>`;
  return box(svg + body);
}

// ---- subscribe ----

const BELL = (c: string, s: number) =>
  `<svg width="${num(s)}" height="${num(s)}" viewBox="0 0 24 24" fill="none" stroke="${c}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="flex:none"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></svg>`;

export function drawSubscribe({ props, w, h, target, local, fps }: Draw) {
  const k = Math.min(w / 560, h / 140);
  const at = Math.max(0, numOr(props.pressAt, 1)) * fps;
  const pressed = local >= at;
  // In: a pop over the first 0.3 s; the press: down and back over 0.25 s.
  const pop = 0.6 + 0.4 * backOut(local / (0.3 * fps));
  const q = (local - at) / (0.25 * fps);
  const press = q >= 0 && q < 1 ? 1 - 0.08 * Math.sin(q * Math.PI) : 1;
  const bg = pressed ? "#3A3A3F" : color(target, props.color, "#FF0033");
  const label = pressed ? str(props.done) : str(props.label);
  const bh = h * 0.8;
  const size = Math.min(bh * 0.38, 46 * k);
  const bell = pressed && props.bell !== false ? BELL("#FFFFFF", size * 1.1) : "";
  // A pointer that glides in and clicks it (D251), for new ones.
  const pointer = props.cursor === true ? (pointerAt(local, at, fps, w, h, w / 2 + size, h / 2, bh * 0.55) ?? "") : "";
  return box(
    `<div data-pressed="${pressed}" style="display:flex;align-items:center;gap:${px(size * 0.5)};height:${px(bh)};padding:0 ${px(bh * 0.45)};border-radius:${px(bh / 2)};background:${bg};color:#FFFFFF;font-family:${DM_SANS};font-weight:700;font-size:${px(size)};letter-spacing:0.01em;white-space:nowrap;box-shadow:0 ${px(bh * 0.08)} ${px(bh * 0.28)} rgb(0 0 0 / 25%);transform:scale(${num(pop * press)})">${bell}<span>${esc(label)}</span></div>${pointer}`,
    "display:flex;align-items:center;justify-content:center",
  );
}

// ---- countdown ----

/** The countdown's number and ring (0–1 left) at `local` frames. */
export function countdownAt(props: { readonly [key: string]: unknown }, local: number, duration: number): { number: number; left: number } {
  const from = Math.max(1, Math.round(numOr(props.from, 5)));
  const left = clamp(1 - local / Math.max(1, duration), 0, 1);
  return { number: Math.ceil(from * left - 1e-9), left };
}

export function drawCountdown(d: Draw) {
  const { props, w, h, target, local } = d;
  const { number, left } = countdownAt(props, local, lengthOf(d));
  const s = Math.min(w, h);
  const c = color(target, props.color, fg(target));
  const track = color(target, props.track, "#FFFFFF2E");
  const t = s * 0.07;
  const r = (s - t) / 2;
  const len = 2 * Math.PI * r;
  const ring =
    props.ring === false
      ? ""
      : `<svg width="${num(s)}" height="${num(s)}" viewBox="0 0 ${num(s)} ${num(s)}" style="position:absolute;left:50%;top:50%;transform:translate(-50%,-50%) rotate(-90deg)"><circle cx="${num(s / 2)}" cy="${num(s / 2)}" r="${num(r)}" fill="none" stroke="${track}" stroke-width="${num(t)}"/><circle cx="${num(s / 2)}" cy="${num(s / 2)}" r="${num(r)}" fill="none" stroke="${c}" stroke-width="${num(t)}" stroke-linecap="round" stroke-dasharray="${num(len)}" stroke-dashoffset="${num(len * (1 - left))}"/></svg>`;
  const digits = String(number).length;
  // Each new number lands with a small pop (D251): 1.18 → 1 over its first quarter second.
  const from = Math.max(1, Math.round(numOr(props.from, 5)));
  const seg = lengthOf(d) / from;
  const within = local - Math.floor(local / seg) * seg;
  const pop = 1 + 0.18 * (1 - out(within / (0.25 * d.fps)));
  return box(`${ring}<div data-count="${number}" style="position:relative;font-family:${DM_SANS};font-weight:700;font-size:${px((s * 0.5) / Math.max(1, digits * 0.6))};line-height:1;color:${c};font-variant-numeric:tabular-nums;transform:scale(${num(pop)})">${number}</div>`, "display:flex;align-items:center;justify-content:center");
}

// ---- sticker ----

/** How long each way in takes (seconds). */
const ENTER_SECONDS: Readonly<Record<string, number>> = { pop: 0.5, draw: 0.8, slide: 0.4, fade: 0.3 };
/** One cycle of each loop (seconds); loops repeat in whole frames so the export draws one cycle's looks. */
const LOOP_SECONDS: Readonly<Record<string, number>> = { wiggle: 0.7, float: 2, pulse: 1, spin: 4 };

/** A sticker's entrance animation; omitted values use the default pop animation. */
export function stickerEnter(props: { readonly [key: string]: unknown }): "pop" | "draw" | "slide" | "fade" | "none" {
  const e = str(props.enter);
  if (e === "pop" || e === "draw" || e === "slide" || e === "fade" || e === "none") return e;
  return "pop";
}

/**
 * A sticker's motion at `local` frames (D251): its way in (pop, draw, slide, fade) over its first
 * moments, then its loop (wiggle, float, pulse, spin), periodic in whole frames from the moment it's
 * in. Scale and rotation in degrees, dy in shares of its size, draw 0–1.
 */
export function stickerMotion(props: { readonly [key: string]: unknown }, local: number, fps: number): { scale: number; rotate: number; dy: number; opacity: number; draw: number } {
  const enter = stickerEnter(props);
  const tilt = clamp(numOr(props.tilt, 0), -45, 45);
  const inFrames = enter === "none" ? 0 : Math.max(1, Math.round(ENTER_SECONDS[enter]! * fps));
  const p = inFrames ? clamp(local / inFrames, 0, 1) : 1;
  let scale = 1;
  let rotate = tilt;
  let dy = 0;
  let opacity = 1;
  if (enter === "pop") {
    const b = backOut(p);
    scale = Math.max(0, b);
    rotate = tilt * b;
  } else if (enter === "slide") {
    dy = (1 - out(p)) * 0.3;
    opacity = out(p);
  } else if (enter === "fade") opacity = out(p);
  const loop = str(props.loop);
  const seconds = LOOP_SECONDS[loop];
  if (seconds) {
    const period = Math.max(2, Math.round(seconds * fps));
    const phase = (Math.max(0, local - inFrames) % period) / period;
    const wave = Math.sin(phase * 2 * Math.PI);
    if (loop === "wiggle") rotate += 7 * wave;
    else if (loop === "float") dy -= 0.04 * wave;
    else if (loop === "pulse") scale *= 1 + 0.07 * wave;
    else if (loop === "spin") rotate += 360 * phase;
  }
  return { scale, rotate, dy, opacity, draw: enter === "draw" ? p : 1 };
}

export function drawSticker({ props, w, h, local, fps }: Draw) {
  const emoji = str(props.emoji) || "🔥";
  const m = stickerMotion(props, local, fps);
  const s = Math.min(w, h);
  const moves = `${m.dy ? `translateY(${px(m.dy * s)}) ` : ""}rotate(${num(m.rotate)}deg) scale(${num(m.scale)})`;
  const fade = m.opacity < 1 ? `;opacity:${num(m.opacity)}` : "";
  // A drawn sticker (D165, D251): our SVG with its colour, words and how much of it is drawn. A mark
  // (a hand-drawn circle, an underline) fills its box; a picture is square in it.
  const art = typeof props.art === "string" ? stickerArt(props.art) : undefined;
  if (art) {
    const svg = artSvg(String(props.art), { color: typeof props.color === "string" ? props.color : undefined, text: typeof props.text === "string" && props.text ? props.text : undefined, draw: m.draw, aspect: w / Math.max(1, h) });
    const [bw, bh] = art.mark ? [w, h] : [s * 0.92, s * 0.92];
    const hidden = m.draw <= 0 ? ";visibility:hidden" : "";
    return box(`<div data-art="${esc(String(props.art))}" style="width:${px(bw)};height:${px(bh)};transform:${moves}${fade}${hidden}">${svg}</div>`, "display:flex;align-items:center;justify-content:center");
  }
  const glyphs = [...emoji].length;
  return box(
    `<div style="font-size:${px((s * 0.8) / Math.max(1, glyphs * 0.85))};line-height:1;font-family:'Apple Color Emoji','Segoe UI Emoji','Noto Color Emoji',${DM_SANS};transform:${moves};white-space:nowrap${fade}">${esc(emoji)}</div>`,
    "display:flex;align-items:center;justify-content:center",
  );
}

// ---- image frame ----

/** A neutral picture glyph for an empty frame (no words). */
const PICTURE = (s: number) =>
  `<svg width="${num(s)}" height="${num(s)}" viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" stroke-opacity="0.45" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.09-3.09a2 2 0 0 0-2.82 0L6 21"/></svg>`;

/** An image frame's fit and crop (D165): cover (fill, cropped) or contain (the whole picture), a zoom 1–4 and its focus in % of the picture. */
export function imageFrameCrop(props: { readonly [key: string]: unknown }): { fit: "cover" | "contain"; zoom: number; focusX: number; focusY: number } {
  return { fit: props.fit === "contain" ? "contain" : "cover", zoom: clamp(numOr(props.zoom, 1), 1, 4), focusX: clamp(numOr(props.focusX, 50), 0, 100), focusY: clamp(numOr(props.focusY, 50), 0, 100) };
}

/**
 * Where an image frame's picture lands in its w × h frame (D165), as CSS object-fit /
 * object-position / scale lay it out: the fitted picture (iw × ih) placed at its focus, then
 * scaled about that point. The frame clips what's outside.
 */
export function imageFrameRect(props: { readonly [key: string]: unknown }, frame: { readonly w: number; readonly h: number }, image: { readonly width: number; readonly height: number }): { x: number; y: number; w: number; h: number } {
  const c = imageFrameCrop(props);
  const k = c.fit === "cover" ? Math.max(frame.w / image.width, frame.h / image.height) : Math.min(frame.w / image.width, frame.h / image.height);
  const w = image.width * k;
  const h = image.height * k;
  const x = (frame.w - w) * (c.focusX / 100);
  const y = (frame.h - h) * (c.focusY / 100);
  // scale about the origin point (focus % of the frame box)
  const ox = frame.w * (c.focusX / 100);
  const oy = frame.h * (c.focusY / 100);
  return { x: ox + (x - ox) * c.zoom, y: oy + (y - oy) * c.zoom, w: w * c.zoom, h: h * c.zoom };
}

export function drawImageFrame({ props, w, h, target }: Draw) {
  const radius = px(clamp(numOr(props.radius, 0), 0, Math.min(w, h) / 2));
  const bw = Math.max(0, numOr(props.borderWidth, 0));
  const border = bw > 0 ? `border:${px(bw)} solid ${color(target, props.border, "#FFFFFF")};` : "";
  const k = Math.min(w, h) / 500;
  const shadow = props.shadow === true ? `box-shadow:0 ${px(18 * k)} ${px(48 * k)} rgb(0 0 0 / 45%);` : "";
  const src = imageSrc(target, props.image);
  const crop = imageFrameCrop(props);
  // Cropped (D165): the picture sits at its focus (object-position) and zooms about it; the frame clips it.
  const cropCss = (crop.focusX !== 50 || crop.focusY !== 50 ? `;object-position:${num(crop.focusX)}% ${num(crop.focusY)}%` : "") + (crop.zoom !== 1 ? `;transform:scale(${num(crop.zoom)});transform-origin:${num(crop.focusX)}% ${num(crop.focusY)}%` : "");
  const inner = src ? `<img src="${src}" alt="" style="display:block;width:100%;height:100%;object-fit:${crop.fit}${cropCss}">` : `<div data-empty="true" style="width:100%;height:100%;display:flex;align-items:center;justify-content:center;background:linear-gradient(135deg,#2A2A2F,#18181B)">${PICTURE(Math.min(w, h) * 0.3)}</div>`;
  return box(inner, `border-radius:${radius};overflow:hidden;${border}${shadow}`);
}
