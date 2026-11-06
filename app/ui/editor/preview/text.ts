import { editorText } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Text and background editing (phase 4 chunk 4, D53): pure planners for the text Inspector, the
// on-canvas drag and inline edit, and the Elements tab's backgrounds. Like the timeline's (D52),
// every gesture becomes one project.edit, so it is validated, undoes in one step and is listed in
// the agent's next note. update_item replaces nested objects whole, so every plan here sends the
// complete style / transform / animation / props, built from the item as it is now.
import { mainEnd as mainEndOf } from "@/app/lib/editor/core/timeline/rules";
import { SOLID, safeMargin, textDecor, textLook, textPlacement, TEXT_STYLES, themeColors, type RenderTarget } from "@/app/lib/editor/core/visual/render";
import { lookItem } from "@/app/lib/editor/core/text/textPlan";
import type { FieldSet, Item, TextBackground, TextShadow, TextStroke, TimedModule } from "@/app/lib/editor/core/timeline/types";

import { findItem, lockOf, type Plan } from "@/app/ui/editor/timeline/edit";
import { layoutOf, uniqueId, type ProjectModel, type Track } from "@/app/lib/editor/state/project";
import { defaultFrames } from "@/app/ui/editor/settings/defaults";

type Problem = { readonly problem: string };
type Style = NonNullable<Item["style"]>;
type Transform = NonNullable<Item["transform"]>;

/**
 * A text item's position, size and box as it's drawn (core textPlacement, the rule textHtml uses:
 * the item's, else its look's, a spanning look's safe area included), weight, colour, alignment.
 */
export function textBox(item: Item, target: Pick<RenderTarget, "canvas" | "theme">) {
  const look = textLook(item.style?.preset);
  const at = textPlacement(item, target);
  return {
    x: Math.round(at.x),
    y: Math.round(at.y),
    w: at.w === undefined ? undefined : Math.round(at.w),
    size: Math.round(at.size),
    weight: item.style?.weight ?? look.weight,
    color: item.style?.color ?? look.color,
    align: item.style?.align ?? look.align ?? "left",
  };
}

/** One update_item on an editable item, or why it can't be edited. */
export function planUpdate(project: ProjectModel, itemId: string, set: FieldSet, summary: string): Plan | Problem {
  const placed = findItem(project, itemId);
  if (!placed) return { problem: "That item isn't in the project any more." };
  const locked = lockOf(placed);
  if (locked) return { problem: locked };
  if (!Object.keys(set).length) return { ops: [], summary };
  return { ops: [{ op: "update_item", item: itemId, set }], summary };
}

/** The item with `id` now, for planners that build a whole nested object from it. */
const itemOf = (project: ProjectModel, id: string) => findItem(project, id)?.item;

/** New text (markup as typed). Unchanged text plans nothing. */
export function planText(project: ProjectModel, itemId: string, text: string): Plan | Problem {
  const item = itemOf(project, itemId);
  if (item && item.text === text) return { ops: [], summary: "" };
  return planUpdate(project, itemId, { text }, "Edit text");
}

const clampWeight = (w: number) => Math.min(900, Math.max(100, Math.round(w / 100) * 100));

/**
 * Style fields merged into the item's style. A null removes the field (so the preset's look
 * applies again). Size is clamped to at least 1 canvas pixel, weight to 100–900 in hundreds,
 * the schema's limits, so a slider can't make an edit the server refuses.
 */
export function planStyle(project: ProjectModel, itemId: string, patch: { readonly [K in keyof Style]?: Style[K] | null }, summary = "Restyle text"): Plan | Problem {
  const item = itemOf(project, itemId);
  if (!item) return { problem: "That item isn't in the project any more." };
  const style: Record<string, unknown> = { ...(item.style ?? {}) };
  for (const [k, v] of Object.entries(patch)) {
    if (v === null || v === undefined) delete style[k];
    else if (k === "size") style[k] = Math.max(1, Math.round(v as number));
    else if (k === "weight") style[k] = clampWeight(v as number);
    else style[k] = v;
  }
  if (JSON.stringify(style) === JSON.stringify(item.style ?? {})) return { ops: [], summary };
  return planUpdate(project, itemId, { style: Object.keys(style).length ? style : null }, summary);
}

/**
 * Apply a look (a built-in text style, D80): the preset and its size and weight (scaled to the
 * canvas); fields the look defines beyond those (font, tracking, line height, case, stroke,
 * shadow, box) are dropped so its look shows. Colour, alignment and position stay: they're the
 * user's (D53). "Reset to preset" (planResetLook) puts those back too.
 */
export function planPreset(project: ProjectModel, itemId: string, preset: string): Plan | Problem {
  const look = TEXT_STYLES[preset];
  if (!look) return { problem: `There's no text style called ${preset}.` };
  return planStyle(project, itemId, { preset, size: project.canvas.height * look.size, weight: look.weight, font: null, tracking: null, lineHeight: null, uppercase: null, stroke: null, shadow: null, background: null }, `Style text as ${look.name}`);
}

/** Reset to preset (D80): the item's look again, whole: its style is the look's (size, weight, every other field from the look) and so is its position. */
export function planResetLook(project: ProjectModel, itemId: string): Plan | Problem {
  const item = itemOf(project, itemId);
  if (!item) return { problem: "That item isn't in the project any more." };
  const look = TEXT_STYLES[item.style?.preset ?? ""];
  if (!look) return { problem: "This text has no preset to reset to: pick one first." };
  const { style, transform } = lookItem(look, project, item.duration);
  if (JSON.stringify(style) === JSON.stringify(item.style ?? {}) && JSON.stringify(transform) === JSON.stringify(item.transform ?? {})) return { ops: [], summary: "" };
  return planUpdate(project, itemId, { style, transform } as FieldSet, `Reset text to ${look.name}`);
}

/** What a text draws now (D80): its stroke, shadow and box in canvas px, the look's where the item sets none. */
export function textDecorOf(item: Item, project: Pick<RenderTarget, "canvas" | "theme">) {
  const look = textLook(item.style?.preset);
  const decor = textDecor(item, look, textBox(item, project).size);
  const shadow: TextShadow | undefined = decor.shadow;
  return { stroke: decor.stroke, shadow, background: decor.background, uppercase: !!(item.style?.uppercase ?? look.uppercase), tracking: item.style?.tracking ?? look.tracking, lineHeight: item.style?.lineHeight ?? look.lineHeight };
}

const r1 = (n: number) => Math.round(n * 10) / 10;
const clamp01 = (n: number) => Math.min(1, Math.max(0, Math.round(n * 100) / 100));

/** A stroke change merged into what's drawn now; false removes it (also a look's), null goes back to the look's. Width ≥ 0 canvas px. */
export function planStroke(project: ProjectModel, itemId: string, patch: Partial<TextStroke> | false | null): Plan | Problem {
  const item = itemOf(project, itemId);
  if (!item) return { problem: "That item isn't in the project any more." };
  if (patch === null || patch === false) return planStyle(project, itemId, { stroke: patch === false ? false : null }, patch === false ? "Remove text outline" : "Use the preset's outline");
  const now = textDecorOf(item, project).stroke;
  const width = Math.max(0, r1(patch.width ?? now?.width ?? Math.max(2, Math.round(textBox(item, project).size * 0.04))));
  const stroke: TextStroke = { width, color: patch.color ?? now?.color ?? "#000000" };
  return planStyle(project, itemId, { stroke }, patch.width !== undefined || !now ? `Outline text: ${width} px` : "Recolour text outline");
}

/** A shadow change merged into what's drawn now; false removes it (including a look's), null goes back to the look's. */
export function planShadow(project: ProjectModel, itemId: string, patch: Partial<TextShadow> | false | null): Plan | Problem {
  const item = itemOf(project, itemId);
  if (!item) return { problem: "That item isn't in the project any more." };
  if (patch === null || patch === false) return planStyle(project, itemId, { shadow: patch === false ? false : null }, patch === false ? "Remove text shadow" : "Use the preset's shadow");
  const now = textDecorOf(item, project).shadow ?? { x: 0, y: 4, blur: 16, color: "#000000", opacity: 0.6 };
  const next = { ...now, ...patch };
  const shadow: TextShadow = { x: r1(next.x ?? 0), y: r1(next.y ?? 0), blur: Math.max(0, r1(next.blur ?? 0)), color: next.color ?? "#000000", opacity: clamp01(next.opacity ?? 1) };
  return planStyle(project, itemId, { shadow }, textDecorOf(item, project).shadow ? "Change text shadow" : "Add text shadow");
}

/** A box behind the text (D80), merged into what's drawn now; false removes it, null goes back to the look's. Padding and radius ≥ 0 canvas px, opacity 0–1. */
export function planTextBox(project: ProjectModel, itemId: string, patch: Partial<TextBackground> | false | null): Plan | Problem {
  const item = itemOf(project, itemId);
  if (!item) return { problem: "That item isn't in the project any more." };
  if (patch === null || patch === false) return planStyle(project, itemId, { background: patch === false ? false : null }, patch === false ? "Remove text box" : "Use the preset's box");
  const size = textBox(item, project).size;
  const was = textDecorOf(item, project).background;
  const now = was ?? { color: "#000000", opacity: 0.7, padding: Math.round(size * 0.35), radius: Math.round(size * 0.18), wrap: "box" as const };
  const next = { ...now, ...patch };
  const background: TextBackground = { color: next.color ?? "#000000", opacity: clamp01(next.opacity ?? 1), padding: Math.max(0, Math.round(next.padding ?? 0)), radius: Math.max(0, Math.round(next.radius ?? 0)), wrap: next.wrap === "lines" ? "lines" : "box" };
  return planStyle(project, itemId, { background }, was ? "Change text box" : `Put text on a box${background.wrap === "lines" ? " per line" : ""}`);
}

/** Position and box width merged into the transform (a null w makes the box fit its text). */
export function planTransform(project: ProjectModel, itemId: string, patch: { readonly x?: number; readonly y?: number; readonly w?: number | null; readonly rotate?: number | null; readonly opacity?: number | null }, summary = "Move text"): Plan | Problem {
  const item = itemOf(project, itemId);
  if (!item) return { problem: "That item isn't in the project any more." };
  // The box as drawn: a look's width (a quote, a spanning title) is written with the position,
  // so moving it doesn't make the text reflow to its natural width.
  const box = textBox(item, project);
  const transform: Record<string, unknown> = { ...(item.transform ?? {}), x: box.x, y: box.y, ...(box.w !== undefined ? { w: box.w } : {}) };
  for (const [k, v] of Object.entries(patch)) {
    if (v === null || v === undefined || (k === "w" && (v as number) <= 0)) delete transform[k];
    // D129: rotation in degrees (to 0.1, within ±180; 0 = none), opacity 0–1 (to 0.01; 1 = none).
    else if (k === "rotate") transform[k] = Math.round((((((v as number) + 180) % 360) + 360) % 360 - 180) * 10) / 10;
    else if (k === "opacity") transform[k] = Math.min(1, Math.max(0, Math.round((v as number) * 100) / 100));
    else transform[k] = Math.round(v as number);
  }
  if (transform.rotate === 0) delete transform.rotate;
  if (transform.opacity === 1) delete transform.opacity;
  if (JSON.stringify(transform) === JSON.stringify(item.transform ?? {})) return { ops: [], summary };
  return planUpdate(project, itemId, { transform: transform as Transform }, summary);
}

/**
 * "Centre on the canvas": the box spans the canvas between the safe margins and the lines are
 * centred, so the text stays centred whatever it says (no measuring involved).
 */
export function planCentre(project: ProjectModel, itemId: string): Plan | Problem {
  const margin = safeMargin(project);
  const item = itemOf(project, itemId);
  if (!item) return { problem: "That item isn't in the project any more." };
  const transform = { ...(item.transform ?? {}), x: margin, y: textBox(item, project).y, w: project.canvas.width - 2 * margin };
  const style = { ...(item.style ?? {}), align: "center" as const };
  return planUpdate(project, itemId, { transform, style }, "Centre text");
}

/** Where "align to the frame" puts a box (D129): an edge or the centre of the safe area, on one axis. */
export type FrameEdge = "left" | "hcenter" | "right" | "top" | "vcenter" | "bottom";

/**
 * Move a text box against the canvas's safe area (D129), like a design tool's align buttons: left /
 * right edge to the margin, top / bottom likewise, or centred on the canvas. `size` is the box as
 * drawn (canvas px, measured), since a text box has no saved height.
 */
export function planFrameAlign(project: ProjectModel, itemId: string, edge: FrameEdge, size: { readonly w: number; readonly h: number }): Plan | Problem {
  const { width, height } = project.canvas;
  const m = safeMargin(project);
  const patch =
    edge === "left" ? { x: m } : edge === "right" ? { x: width - m - size.w } : edge === "hcenter" ? { x: (width - size.w) / 2 } : edge === "top" ? { y: m } : edge === "bottom" ? { y: height - m - size.h } : { y: (height - size.h) / 2 };
  const words: Readonly<Record<FrameEdge, string>> = { left: "left", hcenter: "centre", right: "right", top: "top", vcenter: "middle", bottom: "bottom" };
  return planTransform(project, itemId, patch, `Align text to the ${words[edge]}`);
}

/**
 * Issue #72 (D242): a text resized by a corner handle on the canvas. The text model has no box
 * height and no scale: the drag's factor `k` goes to the font size (style.size, whole px, ≥ 1),
 * and a box width (transform.w, or a look's width, written as a move does) scales with it, so the
 * lines break as before. `drawn` is the box as drawn when the drag began (canvas px, measured); the
 * corner opposite the dragged one stays where it is. One update_item: one edit, one undo.
 */
export function planTextScale(project: ProjectModel, itemId: string, k: number, corner: "nw" | "ne" | "sw" | "se", drawn: { readonly x: number; readonly y: number; readonly w: number; readonly h: number }): Plan | Problem {
  const item = itemOf(project, itemId);
  if (!item) return { problem: "That item isn't in the project any more." };
  const box = textBox(item, project);
  const exact = item.style?.size ?? textPlacement(item, project).size;
  const size = Math.max(1, Math.round(exact * (Number.isFinite(k) && k > 0 ? k : 1)));
  // The factor the whole size really gives, so the box and its anchor move with the text.
  const f = size / exact;
  const left = corner === "nw" || corner === "sw";
  const top = corner === "nw" || corner === "ne";
  // The drawn box may start off the item's x / y (a look drawn elsewhere): keep that offset, scaled.
  const x = left ? drawn.x + drawn.w - drawn.w * f + (box.x - drawn.x) * f : drawn.x + (box.x - drawn.x) * f;
  const y = top ? drawn.y + drawn.h - drawn.h * f + (box.y - drawn.y) * f : drawn.y + (box.y - drawn.y) * f;
  const transform: Record<string, unknown> = { ...(item.transform ?? {}), x: Math.round(x), y: Math.round(y) };
  if (box.w !== undefined) transform.w = Math.max(1, Math.round(box.w * f));
  const style = { ...(item.style ?? {}), size };
  if (size === item.style?.size && JSON.stringify(transform) === JSON.stringify(item.transform ?? {})) return { ops: [], summary: "Resize text" };
  return planUpdate(project, itemId, { style, transform: transform as Transform }, "Resize text");
}

/** Text case (D129): none, capitals (uppercase), lower case or title case. */
export type TextCase = "none" | "upper" | "lower" | "title";
export const caseOf = (item: Item): TextCase => {
  const s = item.style ?? {};
  if (s.case === "lower" || s.case === "title") return s.case;
  return (s.uppercase ?? textLook(s.preset).uppercase) ? "upper" : "none";
};
export function planCase(project: ProjectModel, itemId: string, next: TextCase): Plan | Problem {
  const look = textLook(itemOf(project, itemId)?.style?.preset);
  const patch = next === "upper" ? { uppercase: look.uppercase ? null : true, case: null } : next === "none" ? { uppercase: look.uppercase ? false : null, case: null } : { case: next, uppercase: null };
  const words: Readonly<Record<TextCase, string>> = { none: "normal case", upper: "capitals", lower: "lower case", get title() { return editorText("title case"); } };
  return planStyle(project, itemId, patch as Parameters<typeof planStyle>[2], `Set text in ${words[next]}`);
}

/**
 * One edit over several items (multi-select, D129): each item's plan against the same project,
 * the ops together. A refused one refuses the whole edit (one ⌘Z, all or nothing).
 */
export function planMany(project: ProjectModel, ids: ReadonlyArray<string>, make: (project: ProjectModel, id: string) => Plan | Problem): Plan | Problem {
  const plans = ids.map((id) => make(project, id));
  const bad = plans.find((p): p is Problem => "problem" in p);
  if (bad) return bad;
  const ok = plans as Array<Plan>;
  const ops = ok.flatMap((p) => p.ops);
  const summary = ok.find((p) => p.ops.length)?.summary ?? ok[0]?.summary ?? "";
  return { ops, summary: ids.length > 1 && ops.length ? `${summary} (${ids.length} items)` : summary };
}

export type AnimationSlot = "in" | "out" | "loop";
/**
 * The moves the Inspector offers for In and Out: the built-in move modules the preview draws (core
 * DRAWN_MOVES). One module plays forwards in and backwards out; `out` names it on the Out list (D163).
 */
export const MOVES: ReadonlyArray<{ readonly id: string; readonly label: string; readonly out?: string }> = [
  { id: "", get label() { return editorText("None"); } },
  { id: "move.fade", get label() { return editorText("Fade"); }, out: "Fade out" },
  { id: "move.rise", get label() { return editorText("Rise"); }, out: "Rise out" },
  { id: "move.slide-left", get label() { return editorText("Slide left"); } },
  { id: "move.wipe", get label() { return editorText("Wipe"); }, out: "Wipe out" },
  { id: "move.word-by-word", get label() { return editorText("Word by word"); } },
  { id: "move.zoom", get label() { return editorText("Zoom"); } },
  { id: "move.typewriter", get label() { return editorText("Typewriter"); }, out: "Erase" },
  { id: "move.bounce", get label() { return editorText("Bounce"); }, out: "Bounce out" },
  { id: "move.blur", get label() { return editorText("Blur in"); }, out: "Blur out" },
  { id: "move.scale", get label() { return editorText("Scale in"); }, out: "Scale out" },
  { id: "move.pop", get label() { return editorText("Pop"); }, out: "Pop out" },
];
/** The loop moves (the Loop section, D163): they repeat every loop length while the text shows. */
export const LOOPS: ReadonlyArray<{ readonly id: string; readonly label: string }> = [
  { id: "", get label() { return editorText("None"); } },
  { id: "move.pulse", get label() { return editorText("Pulse"); } },
  { id: "move.wiggle", get label() { return editorText("Wiggle"); } },
];
/** A move's name on a slot's list. */
export const moveLabel = (id: string, slot: AnimationSlot): string => {
  const m: { readonly label: string; readonly out?: string } | undefined = [...MOVES, ...LOOPS].find((x) => x.id === id);
  return m ? (slot === "out" && m.out ? m.out : m.label) : id;
};

/**
 * Set or clear the in or out animation. The duration is clamped to 1 frame…the item's length
 * (the rule the validator checks); clearing the last one removes `animation` altogether.
 */
export function planAnimation(project: ProjectModel, itemId: string, slot: AnimationSlot, move: TimedModule | null): Plan | Problem {
  const item = itemOf(project, itemId);
  if (!item) return { problem: "That item isn't in the project any more." };
  const animation: Record<string, TimedModule> = { ...((item.animation ?? {}) as Record<string, TimedModule>) };
  if (move?.module) {
    const params = move.params && Object.keys(move.params).length ? move.params : undefined;
    const { params: _drop, ...rest } = move;
    animation[slot] = { ...rest, duration: Math.max(1, Math.min(item.duration, Math.round(move.duration))), ...(params ? { params } : {}) };
  }
  else delete animation[slot];
  if (JSON.stringify(animation) === JSON.stringify(item.animation ?? {})) return { ops: [], summary: "" };
  const words = move?.module ? (slot === "loop" ? `${moveLabel(move.module, slot)} loop` : `${moveLabel(move.module, slot)}${slot === "out" && moveLabel(move.module, "out") !== moveLabel(move.module, "in") ? "" : ` ${slot}`}`) : `no ${slot} animation`;
  return planUpdate(project, itemId, { animation: Object.keys(animation).length ? animation : null }, `Animate text: ${words.toLowerCase()}`);
}

export type AnimationChange =
  | { readonly type: "module"; readonly module: string }
  | { readonly type: "duration"; readonly frames: number }
  | { readonly type: "param"; readonly module: string; readonly key: string; readonly value: unknown };

/** Change one animation field per selected item, preserving each item's other settings. */
export function planAnimationChange(project: ProjectModel, itemId: string, slot: AnimationSlot, change: AnimationChange): Plan | Problem {
  const item = itemOf(project, itemId);
  if (!item) return { problem: "That item isn't in the project any more." };
  const current = item.animation?.[slot];
  if (change.type === "module") {
    if (!change.module) return planAnimation(project, itemId, slot, null);
    const duration = current?.duration ?? Math.min(item.duration, slot === "loop" ? project.canvas.fps : Math.round(project.canvas.fps / 3));
    return planAnimation(project, itemId, slot, {
      ...(current?.module === change.module ? current : {}),
      module: change.module,
      duration,
    });
  }
  if (!current) return { ops: [], summary: "" };
  if (change.type === "duration") return planAnimation(project, itemId, slot, { ...current, duration: change.frames });
  if (current.module !== change.module) return { ops: [], summary: "" };
  const params = { ...current.params };
  if (change.value === undefined) delete params[change.key];
  else params[change.key] = change.value;
  return planAnimation(project, itemId, slot, { ...current, params });
}

/** Wrap the selection in [[accent]] markup, or unwrap it if it already is. Returns the new text and selection. */
export function toggleAccent(text: string, from: number, to: number): { text: string; from: number; to: number } {
  const before = text.slice(0, from);
  const inner = text.slice(from, to);
  const after = text.slice(to);
  if (before.endsWith("[[") && after.startsWith("]]")) return { text: before.slice(0, -2) + inner + after.slice(2), from: from - 2, to: to - 2 };
  if (inner.startsWith("[[") && inner.endsWith("]]") && inner.length >= 4) return { text: before + inner.slice(2, -2) + after, from, to: to - 4 };
  if (from === to) {
    // Nothing selected: accent the word under the caret, if any.
    const start = before.search(/\S+$/);
    const end = after.search(/\s|$/);
    if (start === -1 && end <= 0) return { text, from, to };
    const s = start === -1 ? from : start;
    const e = to + Math.max(0, end);
    return toggleAccent(text, s, e);
  }
  return { text: `${before}[[${inner}]]${after}`, from: from + 2, to: to + 2 };
}

/** Colours offered in the Inspector and the Elements tab: the theme's tokens, then a few plain ones. */
export function swatches(project: RenderTarget): Array<{ readonly value: string; readonly css: string; readonly label: string }> {
  const theme = themeColors(project);
  const tokens = Object.entries(theme).map(([token, css]) => ({ value: token, css, label: `Theme ${token}` }));
  const plain = [
    { value: "#FFFFFF", css: "#FFFFFF", get label() { return editorText("White"); } },
    { value: "#000000", css: "#000000", get label() { return editorText("Black"); } },
    { value: "#0B1D3A", css: "#0B1D3A", get label() { return editorText("Dark blue"); } },
    { value: "#F5C26B", css: "#F5C26B", get label() { return editorText("Amber"); } },
  ].filter((p) => !tokens.some((t) => t.css.toLowerCase() === p.css.toLowerCase()));
  return [...tokens, ...plain];
}

/** The Elements tab's backgrounds: a colour or a gradient (component.solid props). */
export const BACKGROUNDS = [
  { id: "theme", get label() { return editorText("Theme"); }, props: { color: "bg" } },
  { id: "black", get label() { return editorText("Black"); }, props: { color: "#000000" } },
  { id: "white", get label() { return editorText("White"); }, props: { color: "#FFFFFF" } },
  { id: "dark-blue", get label() { return editorText("Dark blue"); }, props: { color: "#0B1D3A" } },
  { id: "dusk", get label() { return editorText("Dusk"); }, props: { color: "#0B1D3A", to: "#3B1E54", angle: 160 } },
  { id: "ember", get label() { return editorText("Ember"); }, props: { color: "#1A0F0A", to: "#5A2A12", angle: 180 } },
] as const;
export type BackgroundProps = { readonly color: string; readonly to?: string; readonly angle?: number };

export type PlannedAdd = (Plan & { readonly itemId: string; readonly start: number }) | Problem;

/** An item a background can go behind (D220): a clip with a picture, a text, a graphic or an element; not a sound or captions. */
export function canGoBehind(project: ProjectModel, id: string | undefined): boolean {
  const placed = findItem(project, id);
  if (!placed || placed.track.kind === "audio") return false;
  const { item } = placed;
  if (item.type === "clip") return project.media.find((m) => m.id === item.media)?.kind !== "audio";
  return item.type === "text" || item.type === "graphic" || item.type === "component";
}

/**
 * A background (component.solid), D53 amended by D146 and D220. It is drawn behind everything.
 * Whole edit (`behind` undefined; the Elements tab's default with nothing selected): with the playhead over the
 * footage it spans the whole edit (frame 0 to the end); else (past the footage) the stretch from
 * the playhead to the end, at least a text's default length (F30, 3 s). It goes at the end of the main track
 * when that's where it starts (an empty project, or the playhead at main's end), a card in the sequence;
 * otherwise on a video track below main (the nearest one free for that stretch), so the footage and
 * everything else stay over it; if none is free, a new "Background" track goes directly below main.
 * Behind this (`behind` an item's id, D220): that item's time, on the video track right under the item's
 * (free for that time, not main), else a new "Background" track right there; behind a main clip that's the
 * whole-edit path's tracks below main; behind a text right above main where main ends at the text's start
 * (an empty project's title), a card on main as before. Projects made before keep their tracks.
 */
export function planAddBackground(project: ProjectModel, props: BackgroundProps, frame: number, behind: string | undefined): PlannedAdd {
  const fps = project.canvas.fps;
  const sel = canGoBehind(project, behind) ? findItem(project, behind) : undefined;
  const mainIndex = project.tracks.findIndex((t) => t.role === "main");
  const main = project.tracks[mainIndex];
  if (!main) return { problem: "This project has no main track." };
  const mainEnd = mainEndOf(project);
  const end = layoutOf(project).end;
  const at = Math.max(0, Math.floor(frame));
  let start: number;
  let duration: number;
  if (sel) [start, duration] = [sel.start, sel.item.duration];
  else if (at < mainEnd) [start, duration] = [0, end];
  else [start, duration] = [at, Math.max(defaultFrames("text", fps), end - at)];
  const taken = new Set<string>();
  const itemId = uniqueId(project, "background", taken);
  taken.add(itemId);
  const item: Item = { id: itemId, type: "component", component: SOLID, duration, props: { ...props } };
  const card = () => ({ ops: [{ op: "insert_item" as const, track: main.id, item }], summary: "Add a background", itemId, start });
  const free = (t: Track) => t.kind === "video" && t.role !== "main" && !t.locked && t.items.every((i) => (i.start ?? 0) + i.duration <= start || start + duration <= (i.start ?? 0));
  // Under this track index: the free video track right there (or, under main, the nearest free one
  // below it), else a new one inserted there. Tracks are listed top to bottom and the preview and
  // export stack them in that order, so it draws under that track's items.
  let index: number;
  let track: Track | undefined;
  if (!sel || sel.track.role === "main") {
    if (start === mainEnd && !main.locked && !sel) return card();
    index = mainIndex + 1;
    track = project.tracks.slice(mainIndex + 1).filter((t) => t.kind === "video" && !t.locked).find(free);
  } else {
    index = project.tracks.findIndex((t) => t.id === sel.track.id) + 1;
    const next = project.tracks[index];
    if (next?.role === "main" && start === mainEnd && !main.locked) return card();
    track = next && free(next) ? next : undefined;
  }
  const ops: Array<Plan["ops"][number]> = [];
  let trackId = track?.id;
  if (!trackId) {
    trackId = uniqueId(project, "background-track", taken);
    const names = new Set(project.tracks.map((t) => t.name));
    let name = "Background";
    for (let n = 2; names.has(name); n++) name = `Background ${n}`;
    ops.push({ op: "add_track", index, track: { id: trackId, kind: "video", name } });
  }
  ops.push({ op: "insert_item", track: trackId, item: { ...item, start } });
  return { ops, summary: sel ? `Add a background behind ${sel.item.name ?? sel.item.id}` : "Add a background", itemId, start };
}

/** A background's colours (props merged; a null `to` makes it a plain colour again). */
export function planBackground(project: ProjectModel, itemId: string, patch: { readonly color?: string; readonly to?: string | null; readonly via?: string | null; readonly angle?: number | null; readonly gradient?: "linear" | "radial" | null; readonly noise?: number | null }): Plan | Problem {
  const item = itemOf(project, itemId);
  if (!item) return { problem: "That item isn't in the project any more." };
  const props: Record<string, unknown> = { ...(item.props ?? {}) };
  for (const [k, v] of Object.entries(patch)) {
    if (v === null || v === undefined) delete props[k];
    else props[k] = k === "angle" ? Math.round(v as number) : k === "noise" ? Math.min(1, Math.max(0, Math.round((v as number) * 100) / 100)) : v;
  }
  if (props.noise === 0) delete props.noise;
  if (props.to === undefined) {
    delete props.angle;
    delete props.via;
    delete props.gradient;
  }
  if (props.gradient === "radial") delete props.angle;
  if (JSON.stringify(props) === JSON.stringify(item.props ?? {})) return { ops: [], summary: "" };
  return planUpdate(project, itemId, { props }, "Change the background");
}

export interface Guide {
  readonly axis: "x" | "y";
  /** Canvas pixels. */
  readonly at: number;
  readonly kind: "centre" | "margin";
}

/**
 * Snap a dragged box (canvas pixels) to the canvas's centre lines and safe margins: its centre to
 * the centre, its edges to the margins, whichever is nearest within `threshold`. Returns the
 * snapped corner and the guides to draw.
 */
export function snapBox(box: { x: number; y: number; w: number; h: number }, target: RenderTarget, threshold: number): { x: number; y: number; guides: Array<Guide> } {
  const { width, height } = target.canvas;
  const margin = safeMargin(target);
  const guides: Array<Guide> = [];
  const axis = (pos: number, size: number, full: number, name: "x" | "y") => {
    const candidates: Array<[number, number, Guide["kind"]]> = [
      [full / 2 - size / 2, full / 2, "centre"],
      [margin, margin, "margin"],
      [full - margin - size, full - margin, "margin"],
    ];
    let best: [number, number, Guide["kind"]] | undefined;
    for (const c of candidates) if (Math.abs(c[0] - pos) <= threshold && (!best || Math.abs(c[0] - pos) < Math.abs(best[0] - pos))) best = c;
    if (!best) return pos;
    guides.push({ axis: name, at: best[1], kind: best[2] });
    return best[0];
  };
  return { x: Math.round(axis(box.x, box.w, width, "x")), y: Math.round(axis(box.y, box.h, height, "y")), guides };
}
