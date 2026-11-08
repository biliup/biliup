// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { textLook } from "@/app/lib/editor/core/text/looks";
import type { Project, Op, FieldSet, Item } from "@/app/lib/editor/core/timeline/types";
import { type CaptionMode, CAPTION_DEFAULTS, captionMode } from "@/app/lib/editor/core/text/captions/settings";

// ---- caption styles (F24, D166) ----

/**
 * A caption style: a CapCut-like preset over the caption look (text.caption) that sets the whole
 * style (sizes in shares of the canvas height, turned into canvas px) and, for some, how the words
 * show. Captions-only style keys, drawn by textHtml: `accent` (the speaking word's colour, the
 * karaoke fill), `karaoke` (fill every word said so far), `accentBox` (a box behind the speaking
 * word); `captionStyle` names the preset the gallery shows as chosen.
 */
export interface CaptionStyleDef {
  readonly id: string;
  readonly name: string;
  readonly mode?: CaptionMode;
  readonly style: (h: number) => Record<string, unknown>;
}

const px = (v: number) => Math.round(v * 10) / 10;
const BLACK_STROKE = (h: number, k: number) => ({ width: px(h * k), color: "#000000" });

export const CAPTION_STYLES: ReadonlyArray<CaptionStyleDef> = [
  { id: "box", name: "Box", style: (h) => ({ size: Math.round(h * 0.045), weight: 600 }) },
  { id: "outline", name: "Outline", style: (h) => ({ size: Math.round(h * 0.055), weight: 800, color: "#FFFFFF", stroke: BLACK_STROKE(h, 0.006), shadow: false, background: false }) },
  { id: "karaoke", name: "Karaoke", mode: "word", style: (h) => ({ size: Math.round(h * 0.055), weight: 800, color: "#FFFFFF", stroke: BLACK_STROKE(h, 0.005), shadow: false, background: false, accent: "#FFD23F", karaoke: true }) },
  { id: "highlight", name: "Highlight", mode: "word", style: (h) => ({ size: Math.round(h * 0.055), weight: 800, color: "#FFFFFF", stroke: BLACK_STROKE(h, 0.005), shadow: false, background: false, accent: "#3DDC84" }) },
  { id: "word-box", name: "Word box", mode: "word", style: (h) => ({ size: Math.round(h * 0.055), weight: 800, color: "#FFFFFF", shadow: { x: 0, y: px(h * 0.002), blur: px(h * 0.008), color: "#000000", opacity: 0.6 }, background: false, accent: "#FFFFFF", accentBox: "#7C5CFF" }) },
  { id: "big-yellow", name: "Big yellow", mode: "single", style: (h) => ({ size: Math.round(h * 0.085), weight: 900, uppercase: true, color: "#FFE14D", stroke: BLACK_STROKE(h, 0.008), shadow: false, background: false }) },
  { id: "minimal", name: "Minimal", style: (h) => ({ size: Math.round(h * 0.038), weight: 500, color: "#FFFFFF", shadow: { x: 0, y: px(h * 0.002), blur: px(h * 0.01), color: "#000000", opacity: 0.6 }, background: false }) },
  { id: "card", name: "Card", style: (h) => ({ size: Math.round(h * 0.045), weight: 700, color: "#111111", shadow: false, background: { color: "#FFFFFF", opacity: 0.94, padding: px(h * 0.016), radius: px(h * 0.012), wrap: "lines" } }) },
];

export const captionStyleDef = (id: unknown): CaptionStyleDef | undefined => CAPTION_STYLES.find((s) => s.id === id);

/** A caption style's whole `style` for a canvas (the captions look, the preset's fields, its id). */
export function captionStyleFields(id: string, canvas: { readonly height: number }): Record<string, unknown> | undefined {
  const def = captionStyleDef(id);
  if (!def) return undefined;
  return { preset: CAPTION_DEFAULTS.look, ...def.style(canvas.height), captionStyle: def.id };
}

/**
 * Apply a caption style to a captions item (D166): its whole style (position kept, so a dragged
 * caption stays where it is) and the preset's mode when it has one. One update_item.
 */
export function planCaptionStyle(project: Pick<Project, "tracks" | "canvas">, itemId: string, styleId: string): { readonly ops: Array<Op>; readonly summary: string } | { readonly problem: string } {
  const item = project.tracks.flatMap((t) => t.items).find((i) => i.id === itemId);
  if (!item || item.type !== "captions") return { problem: `"${itemId}" isn't a captions item.` };
  const def = captionStyleDef(styleId);
  const style = captionStyleFields(styleId, project.canvas);
  if (!def || !style) return { problem: `There's no caption style "${styleId}". Styles: ${CAPTION_STYLES.map((s) => s.id).join(", ")}.` };
  const set: Record<string, unknown> = {};
  if (JSON.stringify(style) !== JSON.stringify(item.style ?? {})) set.style = style;
  if (def.mode && def.mode !== captionMode(item)) set.mode = def.mode;
  if (!Object.keys(set).length) return { ops: [], summary: "" };
  return { ops: [{ op: "update_item", item: itemId, set: set as FieldSet }], summary: `Caption style ${def.name.toLowerCase()}` };
}

/** The speaking word's colour (the karaoke fill), its fill mode and box, as the Inspector edits them (null removes the key). */
export function planCaptionHighlight(project: Pick<Project, "tracks">, itemId: string, patch: { readonly accent?: string | null; readonly karaoke?: boolean | null; readonly accentBox?: string | null }): { readonly ops: Array<Op>; readonly summary: string } | { readonly problem: string } {
  const item = project.tracks.flatMap((t) => t.items).find((i) => i.id === itemId);
  if (!item || item.type !== "captions") return { problem: `"${itemId}" isn't a captions item.` };
  const style: Record<string, unknown> = { ...(item.style ?? {}) };
  for (const [k, v] of Object.entries(patch)) {
    if (v === null || v === undefined || v === false) delete style[k];
    else style[k] = v;
  }
  if (JSON.stringify(style) === JSON.stringify(item.style ?? {})) return { ops: [], summary: "" };
  const summary = patch.karaoke !== undefined ? (patch.karaoke ? "Karaoke fill in captions" : "Highlight one word in captions") : patch.accentBox !== undefined ? (patch.accentBox ? "Box the speaking caption word" : "No box behind the caption word") : "Caption highlight colour";
  return { ops: [{ op: "update_item", item: itemId, set: { style: Object.keys(style).length ? style : null } as FieldSet }], summary };
}

/**
 * A short sample of a caption style for its gallery tile (D166): what textHtml draws for a card
 * with the second word speaking, on `canvas`, centred.
 */
export function captionStyleSample(id: string, canvas: { readonly width: number; readonly height: number }, h = 1080): Item | undefined {
  const def = captionStyleDef(id);
  const style = captionStyleFields(id, { height: h });
  if (!def || !style) return undefined;
  const mode = def.mode ?? "line";
  const text = mode === "single" ? "loud" : mode === "line" ? "Say it loud" : style.karaoke ? "[[Say it]] loud" : "Say [[it]] loud";
  const size = Number(style.size);
  const look = textLook(CAPTION_DEFAULTS.look);
  return { id: `sample-${id}`, type: "text", duration: 1, text, style: style as Item["style"], transform: { x: 0, y: Math.round((canvas.height - size * look.lineHeight) / 2), w: canvas.width } };
}
