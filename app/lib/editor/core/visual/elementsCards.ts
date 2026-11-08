// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The follow card and the steps (issue #109, D287): the two built-in components that had manifests
// but weren't drawn. Like the other elements (D130, D251), each is a pure function of its props, its
// box size (canvas px) and the local frame, so the preview and the export draw the same HTML; every
// motion settles to a still within about a second of its last beat, so the export's capped looks
// hold. The follow card's platform mark is the follow handle's (platformMark: brand marks locked to
// their own colour, D275 / D278). Cards have rounded corners, never pill ends; user text is escaped.
import followCardJson from "@/resources/editor/modules/builtin/component.follow-card.json" with { type: "json" };
import stepsJson from "@/resources/editor/modules/builtin/component.steps.json" with { type: "json" };

import { backOut, easeOut, fitSize, inkOn, markInk, platformMark } from "@/app/lib/editor/core/visual/elementsMore";
import { DEFAULT_THEME, DM_SANS, esc, num, resolveColor } from "@/app/lib/editor/core/visual/render";
import { mediaSrc, safeSrc, type Draw } from "@/app/lib/editor/core/visual/components/shared";

export { followCardJson, stepsJson };

const str = (v: unknown) => (typeof v === "string" ? v : "");
const px = (n: number) => `${num(n)}px`;
const numOr = (v: unknown, fallback: number) => (typeof v === "number" && Number.isFinite(v) ? v : fallback);
const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
const len = (s: string) => [...s].length;

// ---- follow card ----

export function drawFollowCard({ props, w, h, target, local, fps }: Draw) {
  const k = Math.min(w / 900, h / 260);
  const logo = safeSrc(mediaSrc(target, props.logo));
  // No platform: the generic @ in `color` (else the theme's accent), no brand.
  const mark = platformMark(target, props, "social");
  const ink = resolveColor(target, props.text, "#0A0A0B");
  const bg = resolveColor(target, props.background, "#FFFFFF");
  const title = str(props.title);
  const handle = str(props.handle);
  const button = str(props.button);
  // In: the card rises (0–0.45 s), the picture pops (0.1–0.5 s), the words slide in (0.25–0.7 s),
  // the button pops last (0.45–0.85 s).
  const rise = easeOut(local / (0.45 * fps));
  const pop = Math.max(0, backOut((local - 0.1 * fps) / (0.4 * fps)));
  const words = easeOut((local - 0.25 * fps) / (0.45 * fps));
  const press = Math.max(0, backOut((local - 0.45 * fps) / (0.4 * fps)));
  const pad = 30 * k;
  const tile = Math.max(1, Math.min(h - 2 * pad, w * 0.4));
  const tileR = tile * 0.24;
  // The picture: your logo on a soft tile with the platform's badge on its corner, else the platform's
  // own mark on its colour.
  let picture: string;
  if (logo) {
    const d = tile * 0.36;
    const badge = `<div data-badge="${esc(mark.platform)}" style="position:absolute;left:${px(tile - d * 0.8)};top:${px(tile - d * 0.8)};width:${px(d)};height:${px(d)};border-radius:50%;background:${mark.tint};box-shadow:0 0 0 ${px(Math.max(1, d * 0.08))} ${bg};display:flex;align-items:center;justify-content:center">${mark.glyph(d, markInk(mark.tint, mark.locked))}</div>`;
    picture = `<div style="position:absolute;inset:0;border-radius:${px(tileR)};background:rgb(128 128 128 / 12%);display:flex;align-items:center;justify-content:center"><img src="${logo}" alt="" style="display:block;width:${px(tile * 0.8)};height:${px(tile * 0.8)};object-fit:contain;border-radius:${px(tileR * 0.6)}"></div>${badge}`;
  } else picture = `<div data-badge="${esc(mark.platform)}" style="position:absolute;inset:0;border-radius:${px(tileR)};background:${mark.tint};display:flex;align-items:center;justify-content:center">${mark.glyph(tile, markInk(mark.tint, mark.locked))}</div>`;
  const pictureHtml = `<div style="position:absolute;left:${px(pad)};top:${px((h - tile) / 2)};width:${px(tile)};height:${px(tile)};transform:scale(${num(pop)})">${picture}</div>`;
  // The button: a rounded rectangle in the platform's colour, the word set to fit.
  const bh = tile * 0.44;
  const bSize = button ? Math.min(bh * 0.4, fitSize(button, bh * 0.4, w * 0.24, 0.58)) : 0;
  const bw = button ? len(button) * 0.58 * bSize + bh * 0.9 : 0;
  const buttonHtml = button
    ? `<div data-button="true" style="position:absolute;right:${px(pad)};top:${px((h - bh) / 2)};width:${px(bw)};height:${px(bh)};border-radius:${px(bh * 0.24)};background:${mark.tint};color:${mark.locked ? markInk(mark.tint, true) : inkOn(mark.tint)};display:flex;align-items:center;justify-content:center;font-family:${DM_SANS};font-weight:700;font-size:${px(bSize)};white-space:nowrap;opacity:${num(clamp(press, 0, 1))};transform:scale(${num(0.6 + 0.4 * press)})">${esc(button)}</div>`
    : "";
  const textX = pad + tile + 30 * k;
  const textW = Math.max(1, w - textX - pad - (button ? bw + 26 * k : 0));
  const titleSize = title ? fitSize(title, 30 * k, textW, 0.56) : 0;
  const handleSize = fitSize(handle, (title ? 54 : 60) * k, textW, 0.56);
  const text = `<div style="position:absolute;left:${px(textX)};top:0;bottom:0;width:${px(textW)};display:flex;flex-direction:column;justify-content:center;gap:${px(6 * k)};font-family:${DM_SANS};color:${ink};opacity:${num(words)};transform:translateX(${px((1 - words) * -24 * k)})">${title ? `<div style="font-weight:600;font-size:${px(titleSize)};line-height:1.15;opacity:0.62;white-space:nowrap">${esc(title)}</div>` : ""}<div style="font-weight:800;font-size:${px(handleSize)};line-height:1.1;letter-spacing:-0.015em;white-space:nowrap">${esc(handle)}</div></div>`;
  return `<div style="position:absolute;inset:0;background:${bg};border-radius:${px(30 * k)};box-shadow:0 ${px(18 * k)} ${px(48 * k)} rgb(0 0 0 / 28%);opacity:${num(rise)};transform:translateY(${px((1 - rise) * 26 * k)})">${pictureHtml}${text}${buttonHtml}</div>`;
}

// ---- steps ----

type Step = { text: string; detail: string };
/** The steps as rows: objects {text, detail}, or plain strings. */
const stepRows = (props: { readonly [key: string]: unknown }): ReadonlyArray<Step> =>
  (Array.isArray(props.steps) ? (props.steps as ReadonlyArray<unknown>) : []).slice(0, 8).map((r) => (typeof r === "string" ? { text: r, detail: "" } : r && typeof r === "object" ? { text: str((r as Step).text), detail: str((r as Step).detail) } : { text: "", detail: "" }));

/**
 * How far each step is at `local` frames: shown (0–1, it slides in at start + i × step) and dimmed
 * (0–1, once the next one is in, when focus isn't false). `line` is how far the line joining the
 * numbers has grown, in rows from the first number.
 */
export function stepsAt(props: { readonly [key: string]: unknown }, local: number, fps: number): { rows: ReadonlyArray<{ shown: number; dimmed: number }>; line: number } {
  const n = stepRows(props).length;
  const start = clamp(numOr(props.start, 0.4), 0, 30) * fps;
  const step = clamp(numOr(props.step, 0.8), 0.1, 10) * fps;
  const at = (i: number) => start + i * step;
  const shown = (i: number) => easeOut((local - at(i)) / (0.4 * fps));
  const rows = Array.from({ length: n }, (_, i) => ({ shown: shown(i), dimmed: props.focus !== false && i + 1 < n ? easeOut((local - at(i + 1)) / (0.3 * fps)) : 0 }));
  let line = 0;
  for (let i = 1; i < n; i++) line += shown(i);
  return { rows, line };
}

export function drawSteps({ props, w, h, target, local, fps }: Draw) {
  const rows = stepRows(props);
  const { rows: states, line } = stepsAt(props, local, fps);
  const k = Math.min(w / 820, h / 560);
  const title = str(props.title);
  const accent = resolveColor(target, props.color, resolveColor(target, "accent", DEFAULT_THEME.colors.accent));
  const bg = resolveColor(target, props.background, "#0A0A0BCC");
  const ink = resolveColor(target, props.text, "#FFFFFF");
  const card = easeOut(local / (0.35 * fps));
  const pad = 44 * k;
  const titleSize = title ? fitSize(title, 46 * k, w - 2 * pad, 0.55) : 0;
  const titleBlock = title ? titleSize * 1.1 + 22 * k : 0;
  const n = Math.max(1, rows.length);
  const rowH = Math.max(1, Math.min(112 * k, (h - 2 * pad - titleBlock) / n));
  const d = Math.min(rowH * 0.6, 62 * k);
  const textX = pad + d + 28 * k;
  const textW = Math.max(1, w - textX - pad);
  const anyDetail = rows.some((r) => r.detail);
  const longest = (key: "text" | "detail") => "x".repeat(rows.reduce((m, r) => Math.max(m, len(r[key])), 1));
  const textSize = Math.min(rowH * (anyDetail ? 0.32 : 0.4), fitSize(longest("text"), 38 * k, textW, 0.55));
  const detailSize = anyDetail ? Math.min(rowH * 0.23, fitSize(longest("detail"), 27 * k, textW, 0.52)) : 0;
  const top = (h - titleBlock - rows.length * rowH) / 2;
  const listTop = top + titleBlock;
  const cx = pad + d / 2;
  const centre = (i: number) => listTop + i * rowH + rowH / 2;
  const numInk = inkOn(accent);
  const stroke = Math.max(1, 4 * k);
  // The line: a faint track joining the numbers, the accent growing down it as steps come in.
  const lineHtml =
    rows.length > 1
      ? `<div style="position:absolute;left:${px(cx - stroke / 2)};top:${px(centre(0))};width:${px(stroke)};height:${px(centre(rows.length - 1) - centre(0))};border-radius:${px(stroke / 2)};background:${ink};opacity:0.16"></div><div data-line="${num(line)}" style="position:absolute;left:${px(cx - stroke / 2)};top:${px(centre(0))};width:${px(stroke)};height:${px(line * rowH)};border-radius:${px(stroke / 2)};background:${accent}"></div>`
      : "";
  const list = rows
    .map((r, i) => {
      const s = states[i]!;
      const fade = num(s.shown * (1 - 0.5 * s.dimmed));
      const badge = `<div style="position:absolute;left:${px(pad)};top:${px(centre(i) - d / 2)};width:${px(d)};height:${px(d)};border-radius:50%;background:${accent};color:${numInk};display:flex;align-items:center;justify-content:center;font-weight:800;font-size:${px(d * 0.46)};font-variant-numeric:tabular-nums;opacity:${num(s.shown)};transform:scale(${num(Math.max(0, backOut(s.shown)))})">${i + 1}</div>`;
      const words = `<div data-step="${i}" data-shown="${s.shown >= 1}" style="position:absolute;left:${px(textX)};top:${px(listTop + i * rowH)};width:${px(textW)};height:${px(rowH)};display:flex;flex-direction:column;justify-content:center;gap:${px(4 * k)};opacity:${fade};transform:translateX(${px((1 - s.shown) * -22 * k)})"><div style="font-weight:700;font-size:${px(textSize)};line-height:1.15;white-space:nowrap">${esc(r.text)}</div>${r.detail ? `<div style="font-weight:500;font-size:${px(detailSize)};line-height:1.2;opacity:0.6;white-space:nowrap">${esc(r.detail)}</div>` : ""}</div>`;
      return badge + words;
    })
    .join("");
  const titleHtml = title ? `<div style="position:absolute;left:${px(pad)};right:${px(pad)};top:${px(top)};font-weight:800;font-size:${px(titleSize)};line-height:1.1;letter-spacing:-0.01em;white-space:nowrap">${esc(title)}</div>` : "";
  return `<div style="position:absolute;inset:0;background:${bg};border-radius:${px(28 * k)};font-family:${DM_SANS};color:${ink};opacity:${num(card)};transform:translateY(${px((1 - card) * 18 * k)})">${titleHtml}${lineHtml}${list}</div>`;
}
