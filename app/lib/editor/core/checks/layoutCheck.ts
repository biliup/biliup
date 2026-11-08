// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Layout checks (D196, issue #36): cheap geometry the critic gets as facts, from the project alone
// (no pixels): a text box or a still clip's box that runs off the canvas, a centred text whose box
// isn't centred, a word wider than its text box, a short label in a fully rounded text background
// (a text box with a background is a pill, never a circle: the "round send button" that drew as an
// oval). Text widths are estimates (an average glyph width in em), so those findings are "soft":
// the critic confirms them on the frames. "hard" ones are exact geometry and fail a review round.
import { textLook } from "@/app/lib/editor/core/text/looks";
import { textPlacement } from "@/app/lib/editor/core/visual/render";
import { textLabel, type CheckRule } from "@/app/lib/editor/core/checks/checks";
import type { Item } from "@/app/lib/editor/core/timeline/types";

/** Average glyph width of a sans face, in em (uppercase wider). */
const EM_LOWER = 0.56;
const EM_UPPER = 0.68;
/** Off-centre between these fractions of the canvas width reads as a mistake (more is a layout choice). */
const CENTRE_MIN = 0.015;
const CENTRE_MAX = 0.12;

const plain = (text: string | undefined) => (text ?? "").replace(/\[\[(.*?)\]\]/gs, "$1").replace(/\*\*(.+?)\*\*/gs, "$1").replace(/`([^`]+)`/g, "$1");
const upper = (item: Item) => item.style?.uppercase === true || textLook(item.style?.preset).uppercase === true;
const emOf = (item: Item) => (upper(item) ? EM_UPPER : EM_LOWER) + Math.max(0, item.style?.tracking ?? 0);

/** A text's box in canvas px: width (given, else estimated from its longest line) and height (lines × line height). */
function textBox(item: Item, project: Parameters<typeof textPlacement>[1]) {
  const { x, y, w, size } = textPlacement(item, project);
  const look = textLook(item.style?.preset);
  const lineHeight = item.style?.lineHeight ?? look.lineHeight;
  const lines = plain(item.text).split("\n");
  const em = emOf(item);
  const longest = Math.max(0, ...lines.map((l) => l.length));
  const width = w ?? Math.min(longest * size * em, project.canvas.width - x);
  // Lines wrap in a given width: count the wrapped lines.
  const count = w ? lines.reduce((n, l) => n + Math.max(1, Math.ceil((l.length * size * em) / Math.max(1, w))), 0) : lines.length;
  return { x, y, w: width, h: count * size * lineHeight, size, given: w !== undefined, align: item.style?.align ?? look.align ?? "left", lineHeight };
}

export const LAYOUT_RULES: ReadonlyArray<CheckRule> = [
  {
    id: "layout-off-canvas",
    title: "Inside the frame",
    description: "A text box or a still clip's box doesn't run off the canvas.",
    severity: "warn",
    check: (ctx) => {
      const { width, height } = ctx.project.canvas;
      const out: Array<{ key: string; message: string; itemIds: Array<string>; frame: number; hint: string }> = [];
      for (const p of ctx.texts) {
        const label = textLabel(p.item.text);
        if (!label) continue;
        const b = textBox(p.item, ctx.project);
        const sides = [b.x < -2 ? "left" : "", b.y < -2 ? "top" : "", b.given && b.x + b.w > width + 2 ? "right" : "", b.y + b.h > height + 2 ? "bottom" : ""].filter(Boolean);
        // Entirely outside is an entrance or exit position, not a cut-off text.
        const gone = b.x >= width || b.y >= height || b.x + b.w <= 0 || b.y + b.h <= 0;
        if (sides.length && !gone) out.push({ key: `layout-off-canvas:${p.item.id}`, message: `"${label}" runs off the ${sides.join(" and ")} edge of the frame.`, itemIds: [p.item.id], frame: p.start, hint: "Move or shrink the text box so all of it is inside the canvas." });
      }
      for (const track of ctx.project.tracks) {
        if (track.hidden || track.kind === "text" || track.kind === "audio") continue;
        for (const item of track.items) {
          const t = item.transform;
          if ((item.type !== "clip" && item.type !== "graphic") || !t || t.x === undefined || t.y === undefined || t.w === undefined || t.h === undefined || item.keys?.length) continue;
          const over = [t.x < -2 ? "left" : "", t.y < -2 ? "top" : "", t.x + t.w > width + 2 ? "right" : "", t.y + t.h > height + 2 ? "bottom" : ""].filter(Boolean);
          const gone = t.x >= width || t.y >= height || t.x + t.w <= 0 || t.y + t.h <= 0;
          if (over.length && !gone) out.push({ key: `layout-off-canvas:${item.id}`, message: `${item.name ?? item.id}'s box runs off the ${over.join(" and ")} edge of the frame.`, itemIds: [item.id], frame: ctx.starts.get(item.id) ?? 0, hint: "Keep the box inside the canvas, or crop it with fit cover." });
        }
      }
      return out;
    },
  },
  {
    id: "layout-off-centre",
    title: "Centred text is centred",
    description: "A text aligned center sits in the middle of the frame (or clearly elsewhere on purpose).",
    severity: "warn",
    check: (ctx) =>
      ctx.texts.flatMap((p) => {
        const label = textLabel(p.item.text);
        if (!label) return [];
        const b = textBox(p.item, ctx.project);
        if (b.align !== "center" || !b.given) return [];
        const off = b.x + b.w / 2 - ctx.project.canvas.width / 2;
        const frac = Math.abs(off) / ctx.project.canvas.width;
        if (frac < CENTRE_MIN || frac > CENTRE_MAX) return [];
        return [{ key: `layout-off-centre:${p.item.id}`, message: `"${label}" is centred in a box ${Math.round(Math.abs(off))} px ${off > 0 ? "right" : "left"} of the frame's centre.`, itemIds: [p.item.id], frame: p.start, hint: `Set x to ${Math.round((ctx.project.canvas.width - b.w) / 2)} (the box centred) or move it clearly off-centre.` }];
      }),
  },
  {
    id: "layout-word-overflow",
    title: "Words fit their box",
    description: "No word is wider than its text box (it would spill out of the box or its background).",
    severity: "warn",
    check: (ctx) =>
      ctx.texts.flatMap((p) => {
        const label = textLabel(p.item.text);
        if (!label) return [];
        const b = textBox(p.item, ctx.project);
        if (!b.given) return [];
        const word = plain(p.item.text).split(/\s+/).reduce((a, x) => (x.length > a.length ? x : a), "");
        const need = word.length * b.size * emOf(p.item);
        if (need <= b.w * 1.05) return [];
        return [{ key: `layout-word-overflow:${p.item.id}`, message: `"${word}" in "${label}" is about ${Math.round(need)} px wide, in a ${Math.round(b.w)} px box.`, itemIds: [p.item.id], frame: p.start, hint: "Widen the box or make the text smaller." }];
      }),
  },
  {
    id: "layout-round-text-box",
    title: "No round text boxes",
    description: "A text background with a radius of half its height is a pill, not a circle: a round button or badge is a graphic.",
    severity: "warn",
    check: (ctx) =>
      ctx.texts.flatMap((p) => {
        const bg = p.item.style?.background;
        if (!bg || typeof bg !== "object" || bg.radius === undefined) return [];
        const label = plain(p.item.text).trim();
        if (!label || label.length > 3) return [];
        const b = textBox(p.item, ctx.project);
        const pad = bg.padding ?? 0;
        const boxH = b.size * b.lineHeight + pad;
        const boxW = (b.given ? b.w : label.length * b.size * emOf(p.item)) + 2 * pad;
        if (bg.radius < boxH / 2 - 1 || Math.abs(boxW - boxH) < boxH * 0.1) return [];
        return [{ key: `layout-round-text-box:${p.item.id}`, message: `"${label}" sits in a ${Math.round(boxW)}×${Math.round(boxH)} px text background with radius ${Math.round(bg.radius)}: it draws as an oval, and the glyph isn't centred in a circle.`, itemIds: [p.item.id], frame: p.start, hint: "Draw a round button as a graphic (an SVG circle with the glyph centred in it)." }];
      }),
  },
];

/** Findings that are exact geometry (fail a review round); the others are estimates the critic confirms. */
export const LAYOUT_HARD: ReadonlySet<string> = new Set(["layout-off-centre", "layout-round-text-box"]);
