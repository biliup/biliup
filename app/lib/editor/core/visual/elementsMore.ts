// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// More general elements (issue #71, D251): a follow handle, a social button (like, bell, follow,
// share, comment, save), a checklist, a quote card, a rating, a price tag, a map pin and before /
// after labels. Like the other elements (D130), each is a pure function of its props, its box size
// (canvas px) and the local frame: the preview and the export draw the same HTML. Every motion
// settles to a still within about a second, or loops in whole frames, so the export has few looks to
// draw. Glyphs are our own SVG, except the follow handle's brand marks (D275,
// brandIcons.ts, from Simple Icons); user text is escaped.
import followJson from "@/resources/editor/modules/builtin/component.follow.json" with { type: "json" };
import socialButtonJson from "@/resources/editor/modules/builtin/component.social-button.json" with { type: "json" };
import checklistJson from "@/resources/editor/modules/builtin/component.checklist.json" with { type: "json" };
import quoteJson from "@/resources/editor/modules/builtin/component.quote.json" with { type: "json" };
import ratingJson from "@/resources/editor/modules/builtin/component.rating.json" with { type: "json" };
import priceTagJson from "@/resources/editor/modules/builtin/component.price-tag.json" with { type: "json" };
import mapPinJson from "@/resources/editor/modules/builtin/component.map-pin.json" with { type: "json" };
import beforeAfterJson from "@/resources/editor/modules/builtin/component.before-after.json" with { type: "json" };

import { BRAND_ICONS } from "@/app/lib/editor/core/visual/brandIcons";
import { DEFAULT_THEME, DM_SANS, esc, num, resolveColor, type RenderTarget } from "@/app/lib/editor/core/visual/render";
import type { Draw } from "@/app/lib/editor/core/visual/components";

export { followJson, socialButtonJson, checklistJson, quoteJson, ratingJson, priceTagJson, mapPinJson, beforeAfterJson };

const str = (v: unknown) => (typeof v === "string" ? v : "");
const px = (n: number) => `${num(n)}px`;
const numOr = (v: unknown, fallback: number) => (typeof v === "number" && Number.isFinite(v) ? v : fallback);
const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
const color = (target: RenderTarget, v: unknown, fallback: string) => resolveColor(target, v, fallback);
const accentOf = (target: RenderTarget) => color(target, "accent", DEFAULT_THEME.colors.accent);
/** Ease out (cubic). */
export const easeOut = (p: number) => 1 - (1 - clamp(p, 0, 1)) ** 3;
/** Ease out with a little overshoot. */
export const backOut = (p: number) => {
  const q = clamp(p, 0, 1);
  const c1 = 1.70158;
  return 1 + (c1 + 1) * (q - 1) ** 3 + c1 * (q - 1) ** 2;
};
const box = (inner: string, extra = "") => `<div style="position:absolute;inset:0;${extra}">${inner}</div>`;
/** Characters, not UTF-16 units (an emoji is one). */
const len = (s: string) => [...s].length;
/**
 * The largest font size (≤ base) at which `text` fits `width` on one line, from DM Sans's average
 * advance (`avg` em per character: ~0.55 for mixed case at 600–700, ~0.66 for capitals at 900).
 * Measured from the string alone, so the preview and the export agree without a layout pass.
 */
export const fitSize = (text: string, base: number, width: number, avg = 0.56) => Math.max(1, Math.min(base, width / Math.max(1, len(text) * avg)));
const TEXT_SHADOW = "0 2px 12px rgb(0 0 0 / 45%)";
/** Dark or white words, whichever reads on a fill (a light accent gets dark words). Non-hex fills get white. */
export function inkOn(fill: string): string {
  const m = /^#([0-9a-f]{3}|[0-9a-f]{6})([0-9a-f]{2})?$/i.exec(fill.trim());
  if (!m) return "#FFFFFF";
  const hex = m[1]!.length === 3 ? [...m[1]!].map((c) => c + c).join("") : m[1]!;
  const [r, g, b] = [0, 2, 4].map((i) => {
    const v = parseInt(hex.slice(i, i + 2), 16) / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b! > 0.45 ? "#0A0A0B" : "#FFFFFF";
}

// ---- glyphs (24 × 24, our own drawings) ----

const glyph = (inner: string, size: number, c: string, extra = "") => `<svg width="${num(size)}" height="${num(size)}" viewBox="0 0 24 24" fill="none" stroke="${c}" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" style="flex:none;display:block${extra}">${inner}</svg>`;
/** A follow handle's generic glyphs (D251): for a kind of platform, or a platform without a brand mark. */
const PLATFORM_GLYPH: Readonly<Record<string, (c: string) => string>> = {
  video: (c) => `<path d="M8.5 6.5v11l9-5.5z" fill="${c}"/>`,
  photos: () => `<rect x="3" y="6.5" width="18" height="13" rx="3.5"/><circle cx="12" cy="13" r="3.6"/><path d="M8.5 6.5l1.5-2.5h4l1.5 2.5"/>`,
  shorts: (c) => `<path d="M10 17.5V5.5l8-1.5v11"/><circle cx="7.5" cy="17.5" r="2.5" fill="${c}"/><circle cx="15.5" cy="15" r="2.5" fill="${c}"/>`,
  social: () => `<circle cx="12" cy="12" r="3.8"/><path d="M15.8 12v1.6a2.6 2.6 0 0 0 5.2 0V12a9 9 0 1 0-3.6 7.2"/>`,
  pro: () => `<rect x="3" y="7.5" width="18" height="12" rx="2.5"/><path d="M9 7.5V5.5a1.5 1.5 0 0 1 1.5-1.5h3A1.5 1.5 0 0 1 15 5.5v2M3 12.5h18"/>`,
  live: (c) => `<circle cx="12" cy="12" r="2.2" fill="${c}"/><path d="M8 8a5.6 5.6 0 0 0 0 8M16 8a5.6 5.6 0 0 1 0 8M5 5a9.9 9.9 0 0 0 0 14M19 5a9.9 9.9 0 0 1 0 14"/>`,
  web: () => `<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c2.6 2.6 3.8 5.6 3.8 9s-1.2 6.4-3.8 9c-2.6-2.6-3.8-5.6-3.8-9S9.4 5.6 12 3z"/>`,
  mail: () => `<rect x="3" y="5.5" width="18" height="13" rx="2.5"/><path d="M3.5 7l8.5 6.5L20.5 7"/>`,
  podcast: () => `<rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21M8.5 21h7"/>`,
};
/** Tints that suggest a kind of platform without being anyone's brand colour. */
const GENERIC_TINT: Readonly<Record<string, string>> = { video: "#F0433A", photos: "#E0457B", shorts: "#14B8A6", social: "#3B82F6", pro: "#2563EB", live: "#8B5CF6", web: "#10B981", mail: "#F59E0B", podcast: "#A855F7" };
/** LinkedIn has no mark in Simple Icons (removed at its request): the briefcase in its blue. */
const LINKEDIN = { title: "LinkedIn", hex: "#0A66C2" };

/**
 * A follow handle's platforms (D275): the brands (their own mark and colour, Simple Icons), LinkedIn,
 * then the generic kinds (D251) as the fallback, with the names the inspector shows.
 */
export const FOLLOW_PLATFORM_NAMES: Readonly<Record<string, string>> = {
  ...Object.fromEntries(Object.entries(BRAND_ICONS).map(([id, b]) => [id, b.title])),
  linkedin: LINKEDIN.title,
  video: "Video (generic)",
  photos: "Photos (generic)",
  shorts: "Shorts (generic)",
  social: "Social (generic)",
  pro: "Work (generic)",
  live: "Live (generic)",
  web: "Website",
  mail: "Email",
  podcast: "Podcast (generic)",
};
const PLATFORM_TINT: Readonly<Record<string, string>> = { ...GENERIC_TINT, ...Object.fromEntries(Object.entries(BRAND_ICONS).map(([id, b]) => [id, b.hex])), linkedin: LINKEDIN.hex };
/** A brand mark: one filled path (not the stroked style of the generic glyphs). */
const brandGlyph = (path: string, size: number, c: string) => `<svg width="${num(size)}" height="${num(size)}" viewBox="0 0 24 24" style="flex:none;display:block"><path fill="${c}" d="${path}"/></svg>`;

/** A social button's glyphs: outline (before the press) and filled (after). */
const BUTTON_GLYPH: Readonly<Record<string, (c: string, filled: boolean) => string>> = {
  like: (c, f) => `<path d="M7 10.5v10H4.5a1.5 1.5 0 0 1-1.5-1.5v-7a1.5 1.5 0 0 1 1.5-1.5zM7 10.5l4-7.5a2.2 2.2 0 0 1 2.6 2.4l-.6 3.6h5.2a2 2 0 0 1 2 2.4l-1.4 7a2.4 2.4 0 0 1-2.4 2.1H7" ${f ? `fill="${c}"` : ""}/>`,
  bell: (c, f) => `<path d="M6 9a6 6 0 0 1 12 0c0 6.5 2.5 8.5 2.5 8.5h-17S6 15.5 6 9z" ${f ? `fill="${c}"` : ""}/><path d="M10.2 20.5a2 2 0 0 0 3.6 0"/>`,
  follow: (c, f) => `<circle cx="9.5" cy="8" r="4" ${f ? `fill="${c}"` : ""}/><path d="M2.5 20.5c0-3.9 3.1-6.5 7-6.5s7 2.6 7 6.5" ${f ? `fill="${c}"` : ""}/><path d="M19 8v6M16 11h6"/>`,
  share: (c, f) => `<path d="M13.5 4.5l7.5 7-7.5 7v-4c-4.6 0-7.8 1.5-10.5 5 .9-5.2 3.7-9.4 10.5-10z" ${f ? `fill="${c}"` : ""}/>`,
  comment: (c, f) => `<path d="M12 3.5c5 0 9 3.3 9 7.6s-4 7.6-9 7.6c-1.2 0-2.3-.2-3.4-.5L3.5 20.5l1.6-4.2C3.8 14.9 3 13.1 3 11.1 3 6.8 7 3.5 12 3.5z" ${f ? `fill="${c}"` : ""}/>`,
  save: (c, f) => `<path d="M6.5 3.5h11a1 1 0 0 1 1 1v16l-6.5-4.5-6.5 4.5v-16a1 1 0 0 1 1-1z" ${f ? `fill="${c}"` : ""}/>`,
};
const BUTTON_WORD: Readonly<Record<string, readonly [string, string]>> = { like: ["Like", "Liked"], bell: ["Notify me", "Notifications on"], follow: ["Follow", "Following"], share: ["Share", "Shared"], comment: ["Comment", "Comment"], save: ["Save", "Saved"] };

/** A mouse pointer (our own), tip at its top-left. */
const POINTER = (s: number) => `<svg width="${num(s)}" height="${num(s)}" viewBox="0 0 24 24" style="display:block;overflow:visible"><path d="M4 2.5v17.2l4.6-4.3 3 6.6 3.3-1.5-3-6.4h6.3z" fill="#FFFFFF" stroke="#111111" stroke-width="1.6" stroke-linejoin="round"/></svg>`;
/**
 * A pointer gliding in from the box's lower right to (tx, ty), clicking at `at` frames and fading
 * after: undefined once it's gone (and before it arrives). All in the box's px.
 */
export function pointerAt(local: number, at: number, fps: number, w: number, h: number, tx: number, ty: number, size: number): string | undefined {
  const t0 = at - 0.7 * fps;
  const t1 = at - 0.12 * fps;
  const gone = at + 0.8 * fps;
  if (local < t0 || local >= gone) return undefined;
  const p = easeOut((local - t0) / (t1 - t0));
  const x = w - size * 0.9 + (tx - (w - size * 0.9)) * p;
  const y = h - size * 0.6 + (ty - (h - size * 0.6)) * p;
  const q = (local - at) / (0.18 * fps);
  const press = q >= 0 && q < 1 ? 1 - 0.18 * Math.sin(q * Math.PI) : 1;
  const fade = local > at + 0.45 * fps ? 1 - (local - at - 0.45 * fps) / (0.35 * fps) : local < t0 + 0.15 * fps ? (local - t0) / (0.15 * fps) : 1;
  return `<div data-pointer="true" style="position:absolute;left:${px(x)};top:${px(y)};opacity:${num(clamp(fade, 0, 1))};transform:scale(${num(press)});transform-origin:0 0;filter:drop-shadow(0 ${px(size * 0.06)} ${px(size * 0.12)} rgb(0 0 0 / 35%))">${POINTER(size)}</div>`;
}

// ---- follow handle ----

/**
 * A platform's mark (D275, D278): its id, whether it's a brand (locked to its own colour), its tint
 * and its glyph at a circle's size. A brand's mark is never recoloured: `color` only tints the
 * generic glyphs. An unknown or missing platform is `fallback` (follow handle: youtube; follow card:
 * the generic @ in `color` or the theme's accent).
 */
export function platformMark(target: RenderTarget, props: { readonly [key: string]: unknown }, fallback: string): { platform: string; locked: boolean; tint: string; glyph: (size: number, ink: string) => string } {
  const platform = PLATFORM_TINT[str(props.platform)] ? str(props.platform) : fallback;
  const brand = BRAND_ICONS[platform];
  const locked = brand !== undefined || platform === "linkedin";
  const tint = locked ? PLATFORM_TINT[platform]! : color(target, props.color, PLATFORM_TINT[platform] ?? accentOf(target));
  const glyphFn = PLATFORM_GLYPH[platform === "linkedin" ? "pro" : platform] ?? PLATFORM_GLYPH.social!;
  return { platform, locked, tint, glyph: (size, ink) => (brand ? brandGlyph(brand.path, size * 0.54, ink) : glyph(glyphFn(ink), size * 0.56, ink)) };
}
/** The ink on a mark's circle: white, or dark on a light colour (pure black for a brand: Snapchat's yellow). */
export const markInk = (tint: string, locked: boolean) => (inkOn(tint) === "#FFFFFF" ? "#FFFFFF" : locked ? "#000000" : "#0A0A0B");

export function drawFollow({ props, w, h, target, local, fps }: Draw) {
  const style = ["pill", "solid", "minimal"].includes(str(props.style)) ? str(props.style) : "pill";
  // Issue #103 (D278): a brand's mark is never recoloured. Its circle (or the solid pill) is the brand's
  // own colour and the mark on it plain white or black; `color` only tints the generic glyphs.
  const { locked, tint, glyph: markGlyph } = platformMark(target, props, "youtube");
  const handle = str(props.handle);
  const label = str(props.label);
  // In: the icon pops (0–0.35 s), the pill opens from it (0.15–0.65 s), the words slide in after.
  const pop = backOut(local / (0.35 * fps));
  const open = easeOut((local - 0.15 * fps) / (0.5 * fps));
  const words = easeOut((local - 0.3 * fps) / (0.45 * fps));
  const pillH = Math.min(h, w * 0.5);
  const icon = pillH * 0.74;
  const pad = (pillH - icon) / 2;
  const textX = pad + icon + pillH * 0.26;
  const textW = Math.max(1, w - textX - pillH * 0.4);
  const big = fitSize(handle, pillH * (label ? 0.34 : 0.4), textW, 0.55);
  const small = fitSize(label, pillH * 0.2, textW, 0.6);
  const bg = style === "solid" ? tint : style === "minimal" ? "transparent" : color(target, props.background, "#FFFFFF");
  const ink = style === "pill" ? color(target, props.text, "#0A0A0B") : style === "solid" ? inkOn(tint) : "#FFFFFF";
  // A light brand colour (Snapchat's yellow, Kick's green) gets dark ink, like the solid pill's words.
  // On the solid pill the circle is white and the mark keeps the brand's colour (black when that's too light).
  const iconBg = style === "solid" ? "#FFFFFF" : tint;
  const iconInk = style === "solid" ? (inkOn(tint) === "#FFFFFF" ? tint : locked ? "#000000" : "#0A0A0B") : markInk(tint, locked);
  const shadow = style === "minimal" ? `text-shadow:${TEXT_SHADOW};` : "";
  const panel = style === "minimal" ? "" : `<div style="position:absolute;left:0;top:${px((h - pillH) / 2)};width:${px(w)};height:${px(pillH)};background:${bg};border-radius:${px(pillH / 2)};box-shadow:0 ${px(pillH * 0.08)} ${px(pillH * 0.3)} rgb(0 0 0 / 22%);clip-path:inset(0 ${num((1 - open) * (1 - pillH / w) * 100)}% 0 0 round ${px(pillH / 2)})"></div>`;
  const glyphHtml = `<div style="position:absolute;left:${px(pad)};top:${px((h - icon) / 2)};width:${px(icon)};height:${px(icon)};border-radius:50%;background:${iconBg};display:flex;align-items:center;justify-content:center;transform:scale(${num(Math.max(0, pop))})">${markGlyph(icon, iconInk)}</div>`;
  const text = `<div style="position:absolute;left:${px(textX)};top:0;bottom:0;width:${px(textW)};display:flex;flex-direction:column;justify-content:center;gap:${px(pillH * 0.02)};font-family:${DM_SANS};color:${ink};${shadow}opacity:${num(words)};transform:translateX(${px((1 - words) * -pillH * 0.3)})">${label ? `<div style="font-weight:600;font-size:${px(small)};line-height:1.1;letter-spacing:0.06em;text-transform:uppercase;opacity:0.7;white-space:nowrap">${esc(label)}</div>` : ""}<div style="font-weight:700;font-size:${px(big)};line-height:1.1;letter-spacing:-0.01em;white-space:nowrap">${esc(handle)}</div></div>`;
  return box(panel + glyphHtml + text);
}

// ---- social button ----

export function drawSocialButton({ props, w, h, target, local, fps }: Draw) {
  const kind = BUTTON_GLYPH[str(props.kind)] ? str(props.kind) : "like";
  const tint = color(target, props.color, "#2F80FF");
  const at = Math.max(0, numOr(props.pressAt, 1)) * fps;
  const pressed = local >= at;
  const words = BUTTON_WORD[kind]!;
  const label = str(props.label) || (pressed ? words[1] : words[0]);
  const rawCount = str(props.count);
  const count = pressed && /^\d{1,9}$/.test(rawCount) ? String(Number(rawCount) + 1) : rawCount;
  const pop = 0.6 + 0.4 * backOut(local / (0.3 * fps));
  const q = (local - at) / (0.25 * fps);
  const press = q >= 0 && q < 1 ? 1 - 0.08 * Math.sin(q * Math.PI) : 1;
  const bh = Math.min(h * 0.82, w * 0.4);
  const icon = bh * 0.46;
  // The words get smaller rather than leaving the box: the longest of both labels and the count.
  const longest = [str(props.label) || words[0], str(props.label) || words[1]].reduce((a, b) => (len(a) > len(b) ? a : b));
  const room = w * 0.96 - bh * 0.72 - icon - (count ? bh * 0.3 : 0);
  const size = Math.min(bh * 0.34, room / Math.max(1, (len(longest) + len(count)) * 0.56));
  const bg = pressed ? tint : "#FFFFFF";
  const ink = pressed ? inkOn(tint) : "#0A0A0B";
  // The burst at the press: six short rays leaving the icon over 0.35 s.
  const b = (local - at) / (0.35 * fps);
  const rays =
    b >= 0 && b < 1
      ? `<svg width="${num(icon * 2.4)}" height="${num(icon * 2.4)}" viewBox="-12 -12 24 24" style="position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);overflow:visible;opacity:${num(1 - b)}">${[0, 60, 120, 180, 240, 300].map((a) => `<path d="M0 ${num(-6 - 4 * b)}V${num(-8.5 - 3 * b)}" stroke="${tint}" stroke-width="1.6" stroke-linecap="round" transform="rotate(${a})"/>`).join("")}</svg>`
      : "";
  const iconHtml = `<div style="position:relative;width:${px(icon)};height:${px(icon)};flex:none">${rays}${glyph(BUTTON_GLYPH[kind]!(ink, pressed), icon, ink, `;transform:scale(${num(pressed && b < 1 ? 1 + 0.25 * Math.sin(clamp(b, 0, 1) * Math.PI) : 1)})`)}</div>`;
  const countHtml = count ? `<span style="font-weight:600;opacity:0.7;padding-left:${px(size * 0.5)};margin-left:${px(size * 0.1)};border-left:${px(Math.max(1, bh * 0.02))} solid currentColor">${esc(count)}</span>` : "";
  const pill = `<div data-pressed="${pressed}" style="display:flex;align-items:center;gap:${px(size * 0.45)};height:${px(bh)};padding:0 ${px(bh * 0.42)} 0 ${px(bh * 0.3)};border-radius:${px(bh / 2)};background:${bg};color:${ink};font-family:${DM_SANS};font-weight:700;font-size:${px(size)};white-space:nowrap;box-shadow:0 ${px(bh * 0.08)} ${px(bh * 0.28)} rgb(0 0 0 / 25%);transform:scale(${num(pop * press)})">${iconHtml}<span>${esc(label)}</span>${countHtml}</div>`;
  const pointer = props.cursor === true ? pointerAt(local, at, fps, w, h, w / 2 - bh * 0.2, h / 2, bh * 0.5) : undefined;
  return box(pill + (pointer ?? ""), "display:flex;align-items:center;justify-content:center");
}

// ---- checklist ----

type Row = { text?: unknown; todo?: unknown };
/** How far each row of a checklist is at `local` frames: shown (0–1) and ticked (0–1). */
export function checklistAt(props: { readonly [key: string]: unknown }, local: number, fps: number): ReadonlyArray<{ shown: number; ticked: number }> {
  const rows = Array.isArray(props.items) ? (props.items as ReadonlyArray<Row>) : [];
  const start = clamp(numOr(props.start, 0.3), 0, 30) * fps;
  const step = clamp(numOr(props.step, 0.6), 0.1, 10) * fps;
  return rows.map((r, i) => {
    const t = start + i * step;
    return { shown: easeOut((local - t + 0.35 * fps) / (0.35 * fps)), ticked: r.todo === true ? 0 : easeOut((local - t) / (0.3 * fps)) };
  });
}

export function drawChecklist({ props, w, h, target, local, fps }: Draw) {
  const rows = (Array.isArray(props.items) ? (props.items as ReadonlyArray<Row>) : []).slice(0, 10);
  const states = checklistAt(props, local, fps);
  const k = Math.min(w / 760, h / 480);
  const title = str(props.title);
  const tick = color(target, props.color, "#3DDC84");
  const bg = color(target, props.background, "#0A0A0BCC");
  const ink = color(target, props.text, "#FFFFFF");
  const pad = 44 * k;
  const titleSize = title ? fitSize(title, 46 * k, w - 2 * pad, 0.55) : 0;
  const avail = h - 2 * pad - (title ? titleSize * 1.5 : 0);
  const rowH = Math.min(78 * k, avail / Math.max(1, rows.length));
  const longest = rows.reduce((n, r) => Math.max(n, len(str(r.text))), 1);
  const size = Math.min(rowH * 0.5, fitSize("x".repeat(longest), 36 * k, w - 2 * pad - rowH, 0.52));
  const boxS = rowH * 0.56;
  const list = rows
    .map((r, i) => {
      const s = states[i]!;
      const done = s.ticked > 0;
      const mark = `<svg width="${num(boxS)}" height="${num(boxS)}" viewBox="0 0 24 24" style="flex:none;display:block"><rect x="1.5" y="1.5" width="21" height="21" rx="6" fill="${done ? tick : "none"}" fill-opacity="${num(s.ticked)}" stroke="${done ? tick : ink}" stroke-opacity="${done ? 1 : 0.5}" stroke-width="2.4"/><path d="M6.5 12.5l3.6 3.6 7.4-8" fill="none" stroke="#FFFFFF" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" pathLength="1" stroke-dasharray="1 1" stroke-dashoffset="${num(1 - s.ticked)}" ${done ? "" : `visibility="hidden"`}/></svg>`;
      const strike = props.strike === true && s.ticked > 0 ? `;text-decoration:line-through;text-decoration-thickness:${px(Math.max(1, size * 0.08))};opacity:${num(1 - 0.35 * s.ticked)}` : "";
      return `<div data-row="${i}" data-ticked="${s.ticked >= 1}" style="display:flex;align-items:center;gap:${px(rowH * 0.3)};height:${px(rowH)};opacity:${num(s.shown)};transform:translateX(${px((1 - s.shown) * -24 * k)})">${mark}<span style="font-weight:600;font-size:${px(size)};line-height:1.15;white-space:nowrap${strike}">${esc(str(r.text))}</span></div>`;
    })
    .join("");
  const titleHtml = title ? `<div style="font-weight:800;font-size:${px(titleSize)};line-height:1.1;letter-spacing:-0.01em;margin-bottom:${px(titleSize * 0.4)};white-space:nowrap">${esc(title)}</div>` : "";
  return box(`<div style="display:flex;flex-direction:column;justify-content:center;height:100%">${titleHtml}${list}</div>`, `padding:${px(pad)};background:${bg};border-radius:${px(28 * k)};font-family:${DM_SANS};color:${ink}`);
}

// ---- quote card ----

/** The size the quote is set at so it fits `width` × `height` in lines of 1.25. */
export function quoteSize(text: string, base: number, width: number, height: number): number {
  let size = base;
  for (let i = 0; i < 40; i++) {
    const perLine = Math.max(1, Math.floor(width / (size * 0.5)));
    const words = text.split(/\s+/).filter(Boolean);
    let lines = 1;
    let used = 0;
    for (const word of words) {
      const n = len(word);
      if (used && used + 1 + n > perLine) {
        lines++;
        used = n;
      } else used += (used ? 1 : 0) + n;
    }
    if (lines * size * 1.25 <= height) return size;
    size *= 0.92;
  }
  return size;
}

export function drawQuote({ props, w, h, target, local, fps }: Draw) {
  const style = ["card", "bold", "minimal"].includes(str(props.style)) ? str(props.style) : "card";
  const quote = str(props.quote);
  const author = str(props.author);
  const role = str(props.role);
  const accent = color(target, props.accent, accentOf(target));
  const k = Math.min(w / 960, h / 440);
  const bg = style === "card" ? color(target, props.background, "#FFFFFF") : style === "bold" ? color(target, props.background, accent) : "transparent";
  const ink = color(target, props.text, style === "card" ? "#0A0A0B" : style === "bold" ? inkOn(bg) : "#FFFFFF");
  const markColor = style === "bold" ? ink : accent;
  const rise = easeOut(local / (0.5 * fps));
  const inner = easeOut((local - 0.15 * fps) / (0.5 * fps));
  const padX = (style === "minimal" ? 48 : 64) * k;
  const padY = 48 * k;
  const attrib = author || role ? 64 * k : 0;
  const size = quoteSize(quote, (style === "bold" ? 66 : 58) * k, w - 2 * padX, h - 2 * padY - attrib - 30 * k);
  const mark = `<svg width="${num(70 * k)}" height="${num(56 * k)}" viewBox="0 0 70 56" style="display:block;margin-bottom:${px(10 * k)};transform:scale(${num(Math.max(0, backOut((local - 0.1 * fps) / (0.4 * fps))))});transform-origin:0 100%"><path d="M0 56V34C0 14 10 3 28 0l3 8c-9 3-14 9-15 18h13v30zM39 56V34C39 14 49 3 67 0l3 8c-9 3-14 9-15 18h13v30z" fill="${markColor}"/></svg>`;
  const shadow = style === "minimal" ? `text-shadow:${TEXT_SHADOW};` : "";
  const bar = style === "minimal" ? `<div style="position:absolute;left:0;top:${px(padY)};bottom:${px(padY)};width:${px(8 * k)};border-radius:${px(4 * k)};background:${accent};transform:scaleY(${num(rise)});transform-origin:50% 0"></div>` : "";
  const who = author || role ? `<div style="display:flex;align-items:baseline;gap:${px(14 * k)};margin-top:${px(22 * k)};white-space:nowrap"><span style="font-weight:700;font-size:${px(32 * k)}">${esc(author)}</span>${role ? `<span style="font-weight:500;font-size:${px(28 * k)};opacity:0.65">${esc(role)}</span>` : ""}</div>` : "";
  const content = `<div style="position:absolute;inset:0;padding:${px(padY)} ${px(padX)};display:flex;flex-direction:column;justify-content:center;font-family:${DM_SANS};color:${ink};${shadow}opacity:${num(inner)};transform:translateY(${px((1 - inner) * 14 * k)})">${mark}<div style="font-weight:${style === "bold" ? 800 : 600};font-size:${px(size)};line-height:1.25;letter-spacing:-0.01em">${esc(quote)}</div>${who}</div>`;
  const panelShadow = style === "minimal" ? "" : `box-shadow:0 ${px(18 * k)} ${px(48 * k)} rgb(0 0 0 / 28%);`;
  return box(bar + content, `background:${bg};border-radius:${px(30 * k)};${panelShadow}opacity:${num(rise)};transform:translateY(${px((1 - rise) * 30 * k)})`);
}

// ---- rating ----

const STAR_PATH = "M12 2.2l2.95 6.3 6.85.8-5.05 4.7 1.35 6.8L12 17.4l-6.1 3.4 1.35-6.8L2.2 9.3l6.85-.8z";

/** How full each star is at `local` frames (0–1 each). */
export function ratingAt(props: { readonly [key: string]: unknown }, local: number, fps: number): ReadonlyArray<number> {
  const stars = clamp(Math.round(numOr(props.stars, 5)), 1, 10);
  const value = clamp(numOr(props.value, 4.5), 0, stars);
  return Array.from({ length: stars }, (_, i) => {
    const full = clamp(value - i, 0, 1);
    if (props.animate === false) return full;
    return full * clamp((local - i * 0.14 * fps) / (0.2 * fps), 0, 1);
  });
}

export function drawRating({ props, w, h, target, local, fps }: Draw) {
  const fills = ratingAt(props, local, fps);
  const label = str(props.label);
  const c = color(target, props.color, "#FFC531");
  const empty = color(target, props.empty, "#FFFFFF40");
  const ink = color(target, props.text, "#FFFFFF");
  const n = fills.length;
  const labelSize = label ? fitSize(label, h * 0.2, w, 0.55) : 0;
  const s = Math.min((h - (label ? labelSize * 1.5 : 0)) * 0.94, w / (n + (n - 1) * 0.12));
  const stars = fills
    .map((f, i) => {
      const pop = props.animate === false ? 1 : 0.7 + 0.3 * backOut((local - i * 0.14 * fps) / (0.3 * fps));
      return `<div data-star="${num(f)}" style="position:relative;width:${px(s)};height:${px(s)};flex:none;transform:scale(${num(pop)})"><svg width="${num(s)}" height="${num(s)}" viewBox="0 0 24 24" style="position:absolute;inset:0;display:block"><path d="${STAR_PATH}" fill="${empty}" stroke-linejoin="round"/></svg><div style="position:absolute;left:0;top:0;bottom:0;width:${num(f * 100)}%;overflow:hidden"><svg width="${num(s)}" height="${num(s)}" viewBox="0 0 24 24" style="display:block"><path d="${STAR_PATH}" fill="${c}" stroke="${c}" stroke-width="0.6" stroke-linejoin="round"/></svg></div></div>`;
    })
    .join("");
  const labelHtml = label ? `<div style="font-family:${DM_SANS};font-weight:600;font-size:${px(labelSize)};line-height:1.2;color:${ink};text-shadow:${TEXT_SHADOW};white-space:nowrap;margin-top:${px(labelSize * 0.3)}">${esc(label)}</div>` : "";
  return box(`<div style="display:flex;gap:${px(s * 0.12)};filter:drop-shadow(0 ${px(s * 0.04)} ${px(s * 0.1)} rgb(0 0 0 / 30%))">${stars}</div>${labelHtml}`, "display:flex;flex-direction:column;align-items:center;justify-content:center");
}

// ---- price tag ----

/** The tag's swing at `local` frames (degrees): it drops in tilted and settles like a hung card. */
export const priceSwing = (local: number, fps: number) => {
  const t = local / fps;
  return t >= 1.4 ? 0 : -16 * Math.exp(-3.6 * t) * Math.cos(8 * t);
};

export function drawPriceTag({ props, w, h, target, local, fps }: Draw) {
  const style = ["tag", "pill", "burst"].includes(str(props.style)) ? str(props.style) : "tag";
  const price = str(props.price);
  const was = str(props.was);
  const label = str(props.label);
  const c = color(target, props.color, "#FF3B30");
  const ink = color(target, props.text, "#FFFFFF");
  const font = `font-family:${DM_SANS};color:${ink};white-space:nowrap`;
  if (style === "burst") {
    const s = Math.min(w, h);
    const pop = Math.max(0, backOut(local / (0.45 * fps)));
    const pts: Array<string> = [];
    for (let i = 0; i < 36; i++) {
      const a = (-90 + i * 10) * (Math.PI / 180);
      const r = i % 2 ? 41 : 50;
      pts.push(`${num(50 + r * Math.cos(a))},${num(50 + r * Math.sin(a))}`);
    }
    const priceSize = fitSize(price, s * 0.3, s * 0.66, 0.58);
    return box(
      `<div style="position:relative;width:${px(s)};height:${px(s)};transform:rotate(${num(-10 * pop)}deg) scale(${num(pop)})"><svg width="${num(s)}" height="${num(s)}" viewBox="0 0 100 100" style="position:absolute;inset:0"><polygon points="${pts.join(" ")}" fill="${c}"/></svg><div style="position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;${font}">${label ? `<div style="font-weight:800;font-size:${px(fitSize(label, s * 0.11, s * 0.6, 0.66))};letter-spacing:0.08em">${esc(label)}</div>` : ""}<div style="font-weight:900;font-size:${px(priceSize)};line-height:1">${esc(price)}</div>${was ? `<div style="font-weight:600;font-size:${px(s * 0.1)};opacity:0.8;text-decoration:line-through">${esc(was)}</div>` : ""}</div></div>`,
      "display:flex;align-items:center;justify-content:center",
    );
  }
  if (style === "pill") {
    const ph = Math.min(h * 0.6, w * 0.34);
    const pop = 0.6 + 0.4 * backOut(local / (0.35 * fps));
    const priceSize = ph * 0.5;
    return box(
      `<div style="display:flex;align-items:center;gap:${px(ph * 0.22)};height:${px(ph)};padding:0 ${px(ph * 0.42)} 0 ${px(label ? ph * 0.14 : ph * 0.42)};border-radius:${px(ph / 2)};background:${c};box-shadow:0 ${px(ph * 0.08)} ${px(ph * 0.3)} rgb(0 0 0 / 25%);${font};transform:scale(${num(pop)})">${label ? `<span style="font-weight:800;font-size:${px(ph * 0.24)};letter-spacing:0.08em;padding:${px(ph * 0.12)} ${px(ph * 0.22)};border-radius:${px(ph / 2)};background:rgb(255 255 255 / 22%)">${esc(label)}</span>` : ""}${was ? `<span style="font-weight:600;font-size:${px(ph * 0.3)};opacity:0.75;text-decoration:line-through">${esc(was)}</span>` : ""}<span style="font-weight:900;font-size:${px(priceSize)}">${esc(price)}</span></div>`,
      "display:flex;align-items:center;justify-content:center",
    );
  }
  // The hang tag: a card with a pointed left end and a hole, swinging from the hole.
  const th = Math.min(h, w * 0.6);
  const tw = Math.min(w, th * 1.75);
  const tip = th * 0.36;
  const hole = { x: tip * 0.62, y: th / 2 };
  const shape = `<svg width="${num(tw)}" height="${num(th)}" viewBox="0 0 ${num(tw)} ${num(th)}" style="position:absolute;inset:0;overflow:visible"><path d="M${num(tip)} 0H${num(tw - th * 0.12)}A${num(th * 0.12)} ${num(th * 0.12)} 0 0 1 ${num(tw)} ${num(th * 0.12)}V${num(th * 0.88)}A${num(th * 0.12)} ${num(th * 0.12)} 0 0 1 ${num(tw - th * 0.12)} ${num(th)}H${num(tip)}L0 ${num(th / 2)}Z" fill="${c}"/><circle cx="${num(hole.x)}" cy="${num(hole.y)}" r="${num(th * 0.065)}" fill="#FFFFFF"/></svg>`;
  const textW = tw - tip - th * 0.22;
  const priceSize = fitSize(price, th * (was || label ? 0.4 : 0.48), textW, 0.58);
  const text = `<div style="position:absolute;left:${px(tip + th * 0.06)};right:${px(th * 0.16)};top:0;bottom:0;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:${px(th * 0.01)};${font}">${label ? `<div style="font-weight:800;font-size:${px(fitSize(label, th * 0.15, textW, 0.66))};letter-spacing:0.1em;opacity:0.9">${esc(label)}</div>` : ""}<div style="font-weight:900;font-size:${px(priceSize)};line-height:1;letter-spacing:-0.02em">${esc(price)}</div>${was ? `<div style="font-weight:600;font-size:${px(th * 0.15)};opacity:0.75;text-decoration:line-through">${esc(was)}</div>` : ""}</div>`;
  const drop = easeOut(local / (0.3 * fps));
  return box(
    `<div style="position:relative;width:${px(tw)};height:${px(th)};opacity:${num(drop)};transform-origin:${px(hole.x)} ${px(hole.y)};transform:translateY(${px((1 - drop) * -th * 0.2)}) rotate(${num(priceSwing(local, fps))}deg);filter:drop-shadow(0 ${px(th * 0.05)} ${px(th * 0.12)} rgb(0 0 0 / 30%))">${shape}${text}</div>`,
    "display:flex;align-items:center;justify-content:center",
  );
}

// ---- map pin ----

/** The pin's drop at `local` frames: its offset (shares of its height, falling to 0 with a bounce) and its squash. */
export function pinDrop(local: number, fps: number): { dy: number; squash: number } {
  const t = local / fps;
  if (t >= 0.7) return { dy: 0, squash: 1 };
  if (t < 0.35) {
    const p = t / 0.35;
    return { dy: -(1 - p * p) * 0.6, squash: 1 };
  }
  const q = (t - 0.35) / 0.35;
  return { dy: -Math.sin(q * Math.PI) * 0.08, squash: 1 - 0.12 * Math.sin(Math.min(1, q * 3) * Math.PI) };
}

export function drawMapPin({ props, w, h, target, local, fps }: Draw) {
  const label = str(props.label);
  const detail = str(props.detail);
  const c = color(target, props.color, "#FF3B30");
  const s = Math.min(w, h);
  const pinH = s * 0.44;
  const pinW = pinH * 0.76;
  const tipY = label || detail ? h * 0.56 : h * 0.72;
  const { dy, squash } = pinDrop(local, fps);
  // The pulse: a ring growing from the tip and fading, every 1.6 s once it has landed.
  const period = Math.max(2, Math.round(1.6 * fps));
  const landed = local - 0.7 * fps;
  const ring = props.pulse === false || landed < 0 ? "" : (() => {
    const p = (landed % period) / period;
    return `<div style="position:absolute;left:${px(w / 2 - pinW * 0.9 * (0.3 + p))};top:${px(tipY - pinW * 0.25 * (0.3 + p))};width:${px(pinW * 1.8 * (0.3 + p))};height:${px(pinW * 0.5 * (0.3 + p))};border-radius:50%;border:${px(s * 0.012)} solid ${c};opacity:${num(0.9 * (1 - p))}"></div>`;
  })();
  const shadowK = 1 + dy * 1.2;
  const shadow = `<div style="position:absolute;left:${px(w / 2 - pinW * 0.32 * shadowK)};top:${px(tipY - pinW * 0.07)};width:${px(pinW * 0.64 * shadowK)};height:${px(pinW * 0.14)};border-radius:50%;background:rgb(0 0 0 / ${num(0.35 * shadowK)})"></div>`;
  const pin = `<svg width="${num(pinW)}" height="${num(pinH)}" viewBox="0 0 38 50" style="position:absolute;left:${px(w / 2 - pinW / 2)};top:${px(tipY - pinH + dy * pinH)};transform:scaleY(${num(squash)});transform-origin:50% 100%;overflow:visible"><path d="M19 49C19 49 2 30.5 2 18.5A17 17 0 0 1 36 18.5C36 30.5 19 49 19 49Z" fill="${c}" stroke="rgb(0 0 0 / 18%)" stroke-width="1.2"/><circle cx="19" cy="18.5" r="7" fill="#FFFFFF"/></svg>`;
  const card = easeOut((local - 0.45 * fps) / (0.4 * fps));
  const cardSize = fitSize(label, s * 0.1, w * 0.86, 0.56);
  const labelHtml =
    label || detail
      ? `<div style="position:absolute;left:0;right:0;top:${px(tipY + s * 0.06)};display:flex;justify-content:center;opacity:${num(card)};transform:translateY(${px((1 - card) * s * 0.04)})"><div style="padding:${px(s * 0.025)} ${px(s * 0.05)};border-radius:${px(s * 0.04)};background:#FFFFFF;color:#0A0A0B;font-family:${DM_SANS};text-align:center;white-space:nowrap;box-shadow:0 ${px(s * 0.015)} ${px(s * 0.05)} rgb(0 0 0 / 28%)">${label ? `<div style="font-weight:700;font-size:${px(cardSize)};line-height:1.15">${esc(label)}</div>` : ""}${detail ? `<div style="font-weight:500;font-size:${px(fitSize(detail, s * 0.065, w * 0.86, 0.55))};line-height:1.2;opacity:0.6">${esc(detail)}</div>` : ""}</div></div>`
      : "";
  return box(ring + shadow + pin + labelHtml);
}

// ---- before / after ----

export function drawBeforeAfter({ props, w, h, target, local, fps }: Draw) {
  const style = ["pill", "tape", "plain"].includes(str(props.style)) ? str(props.style) : "pill";
  const before = str(props.before);
  const after = str(props.after);
  const c = color(target, props.color, "#3DDC84");
  const k = Math.min(w / 1720, h / 900);
  const at = str(props.position) === "bottom" ? "bottom" : "top";
  const size = 54 * k;
  const pad = 40 * k;
  const line = easeOut(local / (0.45 * fps));
  const slide = easeOut((local - 0.15 * fps) / (0.45 * fps));
  const divider = props.divider === false ? "" : `<div style="position:absolute;left:${px(w / 2 - 3 * k)};top:0;width:${px(6 * k)};height:100%;background:#FFFFFF;border-radius:${px(3 * k)};box-shadow:0 0 ${px(16 * k)} rgb(0 0 0 / 40%);transform:scaleY(${num(line)});transform-origin:50% ${at === "top" ? "0" : "100%"}"></div>`;
  const tag = (text: string, fill: string, ink: string, side: "left" | "right") => {
    if (!text) return "";
    const pos = `${side}:${px(pad)};${at}:${px(pad)}`;
    const shift = (1 - slide) * (side === "left" ? -1 : 1) * 40 * k;
    const look =
      style === "plain"
        ? `color:#FFFFFF;text-shadow:${TEXT_SHADOW}`
        : style === "tape"
          ? `background:${fill};color:${ink};padding:${px(10 * k)} ${px(26 * k)};transform:translateX(${px(shift)}) rotate(${side === "left" ? -3 : 3}deg);box-shadow:0 ${px(6 * k)} ${px(18 * k)} rgb(0 0 0 / 25%)`
          : `background:${fill};color:${ink};padding:${px(12 * k)} ${px(30 * k)};border-radius:${px(40 * k)};box-shadow:0 ${px(6 * k)} ${px(18 * k)} rgb(0 0 0 / 25%)`;
    const move = style === "tape" ? "" : `;transform:translateX(${px(shift)})`;
    return `<div data-label="${side}" style="position:absolute;${pos};font-family:${DM_SANS};font-weight:800;font-size:${px(fitSize(text, size, w / 2 - 3 * pad, 0.62))};letter-spacing:0.06em;text-transform:uppercase;line-height:1.1;white-space:nowrap;opacity:${num(slide)};${look}${move}">${esc(text)}</div>`;
  };
  return box(divider + tag(before, "#FFFFFF", "#0A0A0B", "left") + tag(after, c, inkOn(c), "right"));
}
