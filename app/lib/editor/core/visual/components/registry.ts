// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import deviceFrameJson from "@/resources/editor/modules/builtin/component.device-frame.json" with { type: "json" };
import endCardJson from "@/resources/editor/modules/builtin/component.end-card.json" with { type: "json" };
import codeBlockJson from "@/resources/editor/modules/builtin/component.code-block.json" with { type: "json" };
import chartJson from "@/resources/editor/modules/builtin/component.chart.json" with { type: "json" };
import terminalJson from "@/resources/editor/modules/builtin/component.terminal.json" with { type: "json" };
import agentLogJson from "@/resources/editor/modules/builtin/component.agent-log.json" with { type: "json" };
import compareTableJson from "@/resources/editor/modules/builtin/component.compare-table.json" with { type: "json" };
import { shapeJson, drawShape, lowerThirdJson, drawLowerThird, calloutJson, drawCallout, progressBarJson, drawProgressBar, countdownJson, drawCountdown, subscribeJson, drawSubscribe, stickerJson, drawSticker, imageFrameJson, drawImageFrame } from "@/app/lib/editor/core/visual/elements";
import { followJson, drawFollow, socialButtonJson, drawSocialButton, checklistJson, drawChecklist, quoteJson, drawQuote, ratingJson, drawRating, priceTagJson, drawPriceTag, mapPinJson, drawMapPin, beforeAfterJson, drawBeforeAfter } from "@/app/lib/editor/core/visual/elementsMore";
import { followCardJson, drawFollowCard, stepsJson, drawSteps } from "@/app/lib/editor/core/visual/elementsCards";
import { stickerArt } from "@/app/lib/editor/core/visual/stickers";
import { type ClipBox, clipBox, type Rect, baseBox } from "@/app/lib/editor/core/visual/box";
import { SOLID, type RenderTarget, safeMargin, type Motion, num, esc } from "@/app/lib/editor/core/visual/render";
import type { Item, Canvas } from "@/app/lib/editor/core/timeline/types";
import { type ComponentInfo, manifestOf, px } from "@/app/lib/editor/core/visual/components/shared";
import { drawEndCard, drawCodeBlock, drawTerminal, drawChart, drawAgentLog, drawCompareTable } from "@/app/lib/editor/core/visual/components/draw";
import { drawDeviceFrame, deviceSpec, deviceOf } from "@/app/lib/editor/core/visual/components/device";

// ---- the registry ----

const info = (json: unknown, render: ComponentInfo["render"]): ComponentInfo => {
  const manifest = manifestOf(json);
  return { id: manifest.id, name: manifest.name, description: manifest.description, manifest, block: manifest.component, params: manifest.params, render };
};

/** The components the preview and export draw, in the Elements panel's order. */
export const COMPONENTS: ReadonlyArray<ComponentInfo> = [
  // The general elements (D130) first, then the ones made for product videos.
  info(shapeJson, drawShape),
  info(lowerThirdJson, drawLowerThird),
  info(calloutJson, drawCallout),
  info(progressBarJson, drawProgressBar),
  info(countdownJson, drawCountdown),
  info(subscribeJson, drawSubscribe),
  info(stickerJson, drawSticker),
  info(imageFrameJson, drawImageFrame),
  // More general elements (issue #71, D251).
  info(quoteJson, drawQuote),
  info(beforeAfterJson, drawBeforeAfter),
  info(socialButtonJson, drawSocialButton),
  info(followJson, drawFollow),
  // The follow card and the steps (issue #109, D287).
  info(followCardJson, drawFollowCard),
  info(checklistJson, drawChecklist),
  info(stepsJson, drawSteps),
  info(ratingJson, drawRating),
  info(priceTagJson, drawPriceTag),
  info(mapPinJson, drawMapPin),
  info(endCardJson, drawEndCard),
  info(deviceFrameJson, drawDeviceFrame),
  info(codeBlockJson, drawCodeBlock),
  info(terminalJson, drawTerminal),
  info(chartJson, drawChart),
  info(agentLogJson, drawAgentLog),
  info(compareTableJson, drawCompareTable),
];
const BY_ID = new Map(COMPONENTS.map((c) => [c.id, c]));
export const componentInfo = (id: unknown) => (typeof id === "string" ? BY_ID.get(id) : undefined);

/**
 * The Elements tab's sections (issue #71, D251), in order, and each drawn component's section and
 * search words. Built-in components without a category go under "More"; the Background tile
 * sits with the shapes; stickers have their own section with STICKER_GROUPS inside.
 */
export const ELEMENT_CATEGORIES = ["Titles & text", "Social", "Info", "Shapes & media", "Stickers", "Product demos", "More"] as const;
export type ElementCategory = (typeof ELEMENT_CATEGORIES)[number];
const CATEGORY: Readonly<Record<string, readonly [ElementCategory, string]>> = {
  "component.lower-third": ["Titles & text", "name title caption person speaker intro"],
  "component.callout": ["Titles & text", "label pointer arrow annotation look here"],
  "component.quote": ["Titles & text", "quote testimonial review saying words"],
  "component.before-after": ["Titles & text", "before after comparison split side by side"],
  "component.end-card": ["Titles & text", "end outro logo website ending"],
  "component.subscribe": ["Social", "subscribe youtube channel button bell"],
  "component.social-button": ["Social", "like bell follow share comment save button heart"],
  "component.follow": ["Social", "follow handle username account logo youtube instagram tiktok x twitter linkedin facebook threads bluesky mastodon reddit snapchat pinterest twitch kick discord whatsapp telegram github spotify soundcloud podcast vimeo patreon substack medium website email"],
  "component.follow-card": ["Social", "follow card handle end outro logo avatar channel account profile youtube instagram tiktok x twitter linkedin twitch github"],
  "component.progress-bar": ["Info", "progress bar loading timer percent"],
  "component.countdown": ["Info", "countdown timer clock seconds"],
  "component.checklist": ["Info", "checklist list todo steps tick tasks"],
  "component.steps": ["Info", "steps numbered list how to tutorial guide process instructions sequence recipe"],
  "component.rating": ["Info", "rating stars review score"],
  "component.price-tag": ["Info", "price tag sale cost discount shop deal"],
  "component.map-pin": ["Info", "map pin location place travel city"],
  "component.chart": ["Info", "chart bars graph data numbers"],
  "component.shape": ["Shapes & media", "shape rectangle circle line arrow star triangle box"],
  "component.image-frame": ["Shapes & media", "image picture photo frame logo"],
  "component.sticker": ["Stickers", "sticker emoji"],
  "component.device-frame": ["Product demos", "device phone laptop mockup screen"],
  "component.code-block": ["Product demos", "code snippet programming"],
  "component.terminal": ["Product demos", "terminal command line shell"],
  "component.agent-log": ["Product demos", "agent log ai model"],
  "component.compare-table": ["Product demos", "compare table benchmark"],
};
/** A component's Elements section and search words (plugins and unknown ids: "More"). */
export const elementCategory = (id: string): { category: ElementCategory; keywords: ReadonlyArray<string> } => {
  const c = CATEGORY[id];
  return c ? { category: c[0], keywords: c[1].split(" ") } : { category: "More", keywords: [] };
};

/** Every component module the preview and export draw: the registry and the background (render.ts solidHtml). */
export const DRAWN_COMPONENTS: ReadonlyArray<string> = [SOLID, ...COMPONENTS.map((c) => c.id)];

/**
 * Built-in component manifests that are valid in project.json but not drawn yet (as transition.cut
 * isn't): saved, shown to the agent with showsInPreview false, not offered in the panel.
 */
export const UNDRAWN_COMPONENTS: Readonly<Record<string, string>> = {
  "component.zoom-pan": "not drawn: zoom and pan a clip with its scale and x / y keys instead (D75)",
};

/** A drawn component item (not the background, which render.ts draws full-frame). */
export const isDrawnComponent = (item: Pick<Item, "type" | "component">) => item.type === "component" && !!componentInfo(item.component);

/** Canvas pixels per manifest pixel: manifests are sized for a canvas whose shorter side is 1080. */
export const componentScale = (canvas: Pick<Canvas, "width" | "height">) => Math.min(canvas.width, canvas.height) / 1080;

/** The box a component gets with no w and h: the device's frame size, else the manifest's size (scaled), or the canvas. */
export function naturalSize(item: Pick<Item, "component" | "props">, canvas: Pick<Canvas, "width" | "height">): { width: number; height: number } {
  const c = componentInfo(item.component);
  if (!c) return { width: canvas.width, height: canvas.height };
  if (c.block.fullFrame) return { width: canvas.width, height: canvas.height };
  const k = componentScale(canvas);
  // A drawn sticker with its own shape (an underline, a hand-drawn circle; D251) has its own size.
  const art = c.id === "component.sticker" ? stickerArt(item.props?.art)?.size : undefined;
  const size = c.block.screens ? deviceSpec(deviceOf(item.props ?? {})).size : art ? { width: art[0], height: art[1] } : c.block.size;
  return { width: size.width * k, height: size.height * k };
}

/** A component's transform with its default size filled in (no w and h = its natural size). */
const sizedTransform = (item: Pick<Item, "component" | "props" | "transform">, canvas: Pick<Canvas, "width" | "height">) => {
  const size = naturalSize(item, canvas);
  const t = item.transform ?? {};
  const sized = (n: unknown) => typeof n === "number" && Number.isFinite(n) && n > 0;
  return { size, transform: sized(t.w) || sized(t.h) ? t : { ...t, w: size.width, h: size.height } };
};

/** A component's box at `local` frames (D75's box: transform, keys, opacity), or undefined if it isn't one drawn here. */
export function componentBox(item: Pick<Item, "type" | "component" | "props" | "transform" | "keys">, canvas: Pick<Canvas, "width" | "height"> & { readonly fps?: number }, local: number): ClipBox | undefined {
  if (!isDrawnComponent(item)) return undefined;
  const { size, transform } = sizedTransform(item, canvas);
  return clipBox({ transform, keys: item.keys }, canvas, size, local);
}

/** A component's box without keys (where its transform puts it), or undefined if it isn't one drawn here. */
export function componentRest(item: Pick<Item, "type" | "component" | "props" | "transform">, canvas: Pick<Canvas, "width" | "height">): Rect | undefined {
  if (!isDrawnComponent(item)) return undefined;
  const { size, transform } = sizedTransform(item, canvas);
  return baseBox({ transform }, canvas, size);
}

/**
 * Where a new component goes on this canvas (Fable QA round 2, F7, D152): its natural size, made
 * smaller when it wouldn't fit inside the safe area (the canvas less `safeMargin` on every side),
 * placed by its kind: a lower third bottom-left, a subscribe button and a progress bar along the
 * bottom (the bar as wide as the safe area), a sticker top-right, the rest centred. On a portrait
 * canvas the bottom ones sit a tenth of the height higher, clear of the apps' own captions and
 * buttons. Undefined for full-frame components and device frames (their own planner places them).
 */
export function defaultComponentBox(id: string, target: Pick<RenderTarget, "canvas" | "theme">, props?: Record<string, unknown>): { x: number; y: number; w: number; h: number } | undefined {
  const c = componentInfo(id);
  if (!c || c.block.fullFrame || c.block.screens) return undefined;
  const { width: W, height: H } = target.canvas;
  const m = safeMargin(target as RenderTarget);
  const natural = naturalSize({ component: id, props: props ?? componentDefaults(id) }, target.canvas);
  const k = Math.min(1, (W - 2 * m) / natural.width, (H - 2 * m) / natural.height);
  let w = natural.width * k;
  const h = natural.height * k;
  const lift = H > W ? H / 10 : 0;
  let x = (W - w) / 2;
  let y = (H - h) / 2;
  if (id === "component.lower-third") [x, y] = [m, H - m - lift - h];
  else if (id === "component.subscribe" || id === "component.social-button") y = H - m - lift - h;
  else if (id === "component.follow") [x, y] = [m, H - m - lift - h];
  else if (id === "component.progress-bar") [w, x, y] = [W - 2 * m, m, H - m - lift - h];
  // A sticker top-right; a mark (it circles or underlines something) in the middle.
  else if (id === "component.sticker" && !stickerArt(props?.art)?.mark) [x, y] = [W - m - w, m];
  return { x: Math.round(x), y: Math.round(y), w: Math.round(w), h: Math.round(h) };
}

/** The props a new component gets (its manifest's defaults). */
export const componentDefaults = (id: string) => structuredClone((componentInfo(id)?.block.defaults ?? {}) as Record<string, unknown>);

/**
 * A component item as HTML at `local` frames from its start: a wrapper at its box (canvas px) with
 * the opacity given (the caller's: the preview passes everything visualsAt folds in, the export the
 * box's own, its ramps being ffmpeg fades), the in/out animation's motion like text's, and the body.
 * Deterministic: the same item, target and frame give the same string, so the export draws one PNG
 * per distinct look.
 */
export function componentHtml(item: Item, target: RenderTarget, local: number, opts: { opacity?: number; motion?: Motion; fps?: number; box?: ClipBox } = {}): string {
  const parts = componentParts(item, target, local, opts);
  return parts ? wrapComponent(parts.wrapper, parts.body) : "";
}

/**
 * componentHtml in two parts (D76 review): the wrapper (the box's position, size, opacity, scale,
 * the move's offset and reveal) and the body (what the component looks like inside its box). The
 * export caps how many distinct bodies it draws (an animation), never the wrapper's motion, so a
 * keyed box moves on every frame however long it is. `box` is the box to draw it in when a
 * transition moves it (D79: visualsAt's), else its own at `local`.
 */
export function componentParts(item: Item, target: RenderTarget, local: number, { opacity = 1, motion, fps = 30, box: moved }: { opacity?: number; motion?: Motion; fps?: number; box?: ClipBox } = {}): { wrapper: string; body: string } | undefined {
  const c = componentInfo(item.component);
  const box = moved && isDrawnComponent(item) ? moved : componentBox(item, target.canvas, local);
  if (!c || !box) return undefined;
  const body = c.render({ item, props: item.props ?? {}, w: box.w, h: box.h, local, fps, target });
  const css = [`left:${px(box.x)}`, `top:${px(box.y)}`, `width:${px(box.w)}`, `height:${px(box.h)}`, `opacity:${num(opacity)}`];
  const moves: Array<string> = [];
  if (motion?.dx || motion?.dy) moves.push(`translate(${px(motion.dx ?? 0)},${px(motion.dy ?? 0)})`);
  if (box.scale !== 1) moves.push(`scale(${num(box.scale)})`);
  if (moves.length) css.push(`transform:${moves.join(" ")}`);
  if (motion?.reveal !== undefined && motion.reveal < 1) css.push(`clip-path:inset(-20% ${num((1 - motion.reveal) * 100)}% -20% -5%)`);
  return { wrapper: `<div class="pv-comp" data-item="${esc(item.id)}" data-component="${esc(c.id)}" style="${css.join(";")}">`, body };
}

/** A component's wrapper (componentParts) around a body. */
export const wrapComponent = (wrapper: string, body: string) => `${wrapper}${body}</div>`;
