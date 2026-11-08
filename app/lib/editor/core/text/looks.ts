// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Built-in text styles share their manifest definitions with the inspector, renderer and assistant.
import titleJson from "@/resources/editor/modules/builtin/text.title.json" with { type: "json" };
import headerJson from "@/resources/editor/modules/builtin/text.header.json" with { type: "json" };
import lowerThirdJson from "@/resources/editor/modules/builtin/text.lower-third.json" with { type: "json" };
import footnoteJson from "@/resources/editor/modules/builtin/text.footnote.json" with { type: "json" };
import monoLabelJson from "@/resources/editor/modules/builtin/text.mono-label.json" with { type: "json" };
import captionsJson from "@/resources/editor/modules/builtin/text.captions.json" with { type: "json" };
import bigTitleJson from "@/resources/editor/modules/builtin/text.big-title.json" with { type: "json" };
import kickerTitleJson from "@/resources/editor/modules/builtin/text.kicker-title.json" with { type: "json" };
import kickerJson from "@/resources/editor/modules/builtin/text.kicker.json" with { type: "json" };
import highlightJson from "@/resources/editor/modules/builtin/text.highlight.json" with { type: "json" };
import captionJson from "@/resources/editor/modules/builtin/text.caption.json" with { type: "json" };
import strokeTitleJson from "@/resources/editor/modules/builtin/text.stroke-title.json" with { type: "json" };
import outlineJson from "@/resources/editor/modules/builtin/text.outline.json" with { type: "json" };
import boxedLabelJson from "@/resources/editor/modules/builtin/text.boxed-label.json" with { type: "json" };
import quoteJson from "@/resources/editor/modules/builtin/text.quote.json" with { type: "json" };
import chapterJson from "@/resources/editor/modules/builtin/text.chapter.json" with { type: "json" };
import wordByWordJson from "@/resources/editor/modules/builtin/move.word-by-word.json" with { type: "json" };
import zoomJson from "@/resources/editor/modules/builtin/move.zoom.json" with { type: "json" };
import pulseJson from "@/resources/editor/modules/builtin/move.pulse.json" with { type: "json" };
import wiggleJson from "@/resources/editor/modules/builtin/move.wiggle.json" with { type: "json" };

/** A look's stroke: width in em of the font size. */
export interface LookStroke {
  readonly width: number;
  readonly color?: string;
}
/** A look's shadow: offsets and blur in em of the font size. */
export interface LookShadow {
  readonly x?: number;
  readonly y?: number;
  readonly blur?: number;
  readonly color?: string;
  readonly opacity?: number;
}
/** A look's background box: padding and radius in em of the font size. */
export interface LookBackground {
  readonly color: string;
  readonly opacity?: number;
  readonly padding?: number;
  readonly radius?: number;
  readonly wrap?: "box" | "lines";
}
export interface LookMove {
  readonly module: string;
  readonly seconds: number;
  readonly params?: { readonly [key: string]: unknown };
}

/**
 * How a text style looks. size and y are shares of the canvas
 * height, x and w of its width; stroke, shadow and background lengths are em of the font size.
 */
export interface TextLook {
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly size: number;
  readonly weight: number;
  readonly font: string;
  readonly tracking: number;
  readonly lineHeight: number;
  readonly color: string;
  readonly align?: "left" | "center" | "right";
  readonly uppercase?: boolean;
  readonly x: number;
  readonly y: number;
  readonly w?: number;
  readonly span?: boolean;
  readonly stroke?: LookStroke;
  readonly shadow?: LookShadow;
  readonly background?: LookBackground;
  readonly text: string;
  readonly animation?: { readonly in?: LookMove; readonly out?: LookMove };
  readonly with?: ReadonlyArray<{ readonly look: string; readonly text: string; readonly dy?: number }>;
}

type LookManifest = { readonly id: string; readonly name: string; readonly description?: string; readonly look: Omit<TextLook, "id" | "name" | "description" | "font" | "tracking" | "lineHeight" | "color" | "text"> & Partial<Pick<TextLook, "font" | "tracking" | "lineHeight" | "color" | "text">> };

const lookOf = (json: unknown): TextLook => {
  const m = json as LookManifest;
  return { font: "sans", tracking: 0, lineHeight: 1.1, color: "fg", text: m.name, ...m.look, id: m.id, name: m.name, description: m.description ?? "" } as TextLook;
};

/**
 * Every built-in look by module id, in the Inspector's order: the six from before D80 first, then
 * the curated ones (D80).
 */
export const TEXT_STYLES: Readonly<Record<string, TextLook>> = Object.fromEntries(
  [titleJson, headerJson, lowerThirdJson, footnoteJson, monoLabelJson, captionsJson, bigTitleJson, kickerTitleJson, kickerJson, highlightJson, captionJson, strokeTitleJson, outlineJson, boxedLabelJson, quoteJson, chapterJson].map((j) => {
    const look = lookOf(j);
    return [look.id, look];
  }),
);


/** Default typography for a text item without a preset. */
export const PLAIN_LOOK: TextLook = { id: "", name: "Text", description: "", size: 0.08, weight: 600, font: "sans", tracking: -0.02, lineHeight: 1.05, color: "fg", x: 0.08, y: 0.42, text: "Text" };

export const textLook = (preset: unknown): TextLook => (typeof preset === "string" && TEXT_STYLES[preset]) || PLAIN_LOOK;

/**
 * The Text tab's grid (D80): the three tiles from before (same ids, so "title" still adds a
 * text.title), then the curated looks. Each tile adds its look (and the looks in its `with`).
 */
export const TEXT_PRESETS = [
  { id: "title", module: "text.title", label: "Title", text: "Title" },
  { id: "text", module: "text.header", label: "Text", text: "Text" },
  { id: "lower-third", module: "text.lower-third", label: "Lower third", text: "Lower third" },
  ...["text.big-title", "text.kicker-title", "text.highlight", "text.caption", "text.stroke-title", "text.outline", "text.boxed-label", "text.quote", "text.chapter"].map((module) => {
    const look = TEXT_STYLES[module]!;
    return { id: module.replace(/^text\./, ""), module, label: look.name, text: look.text };
  }),
] as const satisfies ReadonlyArray<{ id: string; module: string; label: string; text: string }>;
/** A built-in text preset offered by the Text tab. */
export type TextPreset = { readonly id: string; readonly module: string; readonly label: string; readonly text: string };

// ---- text moves (D81) ----

export const WORD_BY_WORD = "move.word-by-word";
export const ZOOM_MOVE = "move.zoom";
export type WordStyle = "pop" | "fade" | "rise";
export const WORD_STYLES: ReadonlyArray<WordStyle> = (wordByWordJson as { params: { properties: { style: { enum: Array<WordStyle> } } } }).params.properties.style.enum;
const zoomParams = (zoomJson as { params: { properties: { scale: { default: number }; ease: { default: string } } } }).params.properties;
/** move.zoom's defaults (its manifest's): the scale it starts from (in) or ends at (out), and the ease. */
export const ZOOM_DEFAULTS = { scale: zoomParams.scale.default, ease: zoomParams.ease.default as "linear" | "in" | "out" | "inOut" };

// ---- more text moves (F21, D163) ----

export const TYPEWRITER = "move.typewriter";
export const BOUNCE_MOVE = "move.bounce";
export const BLUR_MOVE = "move.blur";
export const SCALE_MOVE = "move.scale";
export const POP_MOVE = "move.pop";
/** Loop moves (the animation's `loop` slot): they repeat every loop length for as long as the text shows. */
export const PULSE_MOVE = "move.pulse";
export const WIGGLE_MOVE = "move.wiggle";
export const LOOP_MOVES = [PULSE_MOVE, WIGGLE_MOVE] as const;
/** The loop moves' default amount (their manifests'): pulse's growth share, wiggle's largest tilt in degrees. */
export const LOOP_DEFAULTS: Readonly<Record<string, number>> = {
  [PULSE_MOVE]: (pulseJson as { params: { properties: { amount: { default: number } } } }).params.properties.amount.default,
  [WIGGLE_MOVE]: (wiggleJson as { params: { properties: { amount: { default: number } } } }).params.properties.amount.default,
};
